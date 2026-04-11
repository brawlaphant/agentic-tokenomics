import Anthropic from "@anthropic-ai/sdk";
import { config } from "./config.js";
import type {
  CreditClass,
  Project,
  CreditBatch,
  ScreeningResult,
} from "./types.js";

const client = new Anthropic({ apiKey: config.anthropicApiKey });

const SYSTEM_PROMPT = `You are AGENT-001, the Registry Reviewer for Regen Network.

Your responsibilities:
1. Pre-screening credit class applications for methodology quality and completeness
2. Validating project registrations against their credit class requirements
3. Reviewing credit batch issuances for accuracy, anomalies, and compliance
4. Flagging potential issues before they reach on-chain governance

Core Principles:
- NEVER approve or reject unilaterally — provide scoring and recommendations
- Be conservative: false positives (flagging good items) are better than false negatives (missing bad ones)
- Consider methodology rigor, issuer reputation, data completeness, and novelty
- Cite specific concerns with evidence from the data
- Be precise with scores and confidence levels

Scoring Rubric (each factor 0–1000):
- Methodology Quality (40% weight): Rigor of the credit methodology, scientific basis, measurement/reporting/verification approach
- Reputation (30% weight): Track record of admin/issuer addresses, history on Regen Network, known entities
- Novelty (20% weight): Innovation in approach, new credit types, geographic expansion, methodological advances
- Completeness (10% weight): Metadata quality, required fields present, documentation thoroughness

Composite Score = weighted sum of factors (0–1000)
- >= 700: Recommend APPROVE
- 300–699: Recommend CONDITIONAL (needs human review)
- < 300: Recommend REJECT

Confidence (0–1000): How certain you are in your assessment given available data.

Regen Network Context:
- Cosmos SDK-based blockchain for ecological assets (eco-credits)
- Credit classes define methodologies (e.g., C01 for carbon, C02 for forestry)
- Projects register under a class and represent real-world ecological activity
- Batches are issuances of credits for a project over a date range
- x/ecocredit module manages classes, projects, batches, and retirements

Output Format:
Always respond with valid JSON matching this schema:
{
  "score": <number 0-1000>,
  "confidence": <number 0-1000>,
  "recommendation": "APPROVE" | "CONDITIONAL" | "REJECT",
  "factors": {
    "methodology_quality": <number 0-1000>,
    "reputation": <number 0-1000>,
    "novelty": <number 0-1000>,
    "completeness": <number 0-1000>
  },
  "rationale": "<markdown string with detailed reasoning>"
}`;

/**
 * Screen a credit class application via Claude.
 */
export async function screenCreditClass(
  classData: CreditClass,
  issuers: string[],
  existingClasses: CreditClass[]
): Promise<ScreeningResult> {
  const prompt = `Screen this credit class registration for Regen Network.

## Credit Class Data
- Class ID: ${classData.id}
- Admin: ${classData.admin}
- Credit Type: ${classData.credit_type?.name || "unknown"} (${classData.credit_type?.abbreviation || "?"})
- Unit: ${classData.credit_type?.unit || "unknown"}
- Precision: ${classData.credit_type?.precision ?? "unknown"}
- Metadata: ${classData.metadata || "(empty)"}

## Authorized Issuers (${issuers.length})
${issuers.length > 0 ? issuers.map((addr) => `- ${addr}`).join("\n") : "- None registered"}

## Existing Credit Classes on Network (${existingClasses.length} total)
${existingClasses
  .slice(0, 20)
  .map(
    (c) =>
      `- ${c.id}: ${c.credit_type?.name || "unknown"} (admin: ${c.admin})`
  )
  .join("\n")}
${existingClasses.length > 20 ? `\n... and ${existingClasses.length - 20} more` : ""}

Evaluate:
1. Is the credit type well-defined with appropriate unit and precision?
2. Does the metadata provide sufficient methodology documentation?
3. Is this duplicating an existing class, or does it bring something new?
4. Are the admin and issuers known entities on the network?
5. Are there any red flags (e.g., empty metadata, suspicious addresses)?

Respond with the JSON screening result.`;

  return callClaude(prompt);
}

/**
 * Screen a project registration via Claude.
 */
export async function screenProject(
  projectData: Project,
  classData: CreditClass | null
): Promise<ScreeningResult> {
  const prompt = `Screen this project registration for Regen Network.

## Project Data
- Project ID: ${projectData.id}
- Class ID: ${projectData.class_id}
- Admin: ${projectData.admin}
- Jurisdiction: ${projectData.jurisdiction || "(not specified)"}
- Reference ID: ${projectData.reference_id || "(none)"}
- Metadata: ${projectData.metadata || "(empty)"}

## Parent Credit Class
${
  classData
    ? `- Class ID: ${classData.id}
- Credit Type: ${classData.credit_type?.name || "unknown"} (${classData.credit_type?.abbreviation || "?"})
- Class Admin: ${classData.admin}
- Class Metadata: ${classData.metadata || "(empty)"}`
    : "- Class data not available"
}

Evaluate:
1. Does the project align with its credit class methodology?
2. Is the jurisdiction specified and reasonable for this credit type?
3. Does the metadata contain adequate project documentation?
4. Is the project admin a known entity or connected to the class admin/issuers?
5. Does the reference ID link to external verification (e.g., a registry)?
6. Are there any red flags (e.g., missing jurisdiction, empty metadata)?

Respond with the JSON screening result.`;

  return callClaude(prompt);
}

/**
 * Screen a credit batch issuance via Claude.
 */
export async function screenBatch(
  batchData: CreditBatch,
  projectData: Project | null
): Promise<ScreeningResult> {
  const prompt = `Screen this credit batch issuance for Regen Network.

## Batch Data
- Batch Denom: ${batchData.denom}
- Project ID: ${batchData.project_id}
- Issuer: ${batchData.issuer}
- Start Date: ${batchData.start_date}
- End Date: ${batchData.end_date}
- Total Amount: ${batchData.total_amount}
- Open: ${batchData.open}
- Metadata: ${batchData.metadata || "(empty)"}

## Parent Project
${
  projectData
    ? `- Project ID: ${projectData.id}
- Class ID: ${projectData.class_id}
- Jurisdiction: ${projectData.jurisdiction || "(not specified)"}
- Project Admin: ${projectData.admin}
- Project Metadata: ${projectData.metadata || "(empty)"}`
    : "- Project data not available"
}

Evaluate:
1. Is the issuance amount reasonable for the credit type and date range?
2. Does the date range (start to end) make sense for ecological credit generation?
3. Is the issuer authorized for this project's credit class?
4. Does the batch metadata provide adequate supporting evidence?
5. Are there anomalies? (e.g., very large issuance, very short date range, open batch with large amount)
6. Is the batch consistent with the parent project's jurisdiction and methodology?

Respond with the JSON screening result.`;

  return callClaude(prompt);
}

// ── Helpers ──────────────────────────────────────────────────

async function callClaude(prompt: string): Promise<ScreeningResult> {
  const response = await client.messages.create({
    model: config.model,
    max_tokens: 2000,
    system: SYSTEM_PROMPT,
    messages: [{ role: "user", content: prompt }],
  });

  const text = response.content
    .filter((b): b is Anthropic.TextBlock => b.type === "text")
    .map((b) => b.text)
    .join("\n");

  return parseScreeningResult(text);
}

function parseScreeningResult(text: string): ScreeningResult {
  // Try to extract JSON from the response (handles markdown code blocks)
  const jsonMatch = text.match(/\{[\s\S]*\}/);
  if (!jsonMatch) {
    return fallbackResult(text);
  }

  try {
    const parsed = JSON.parse(jsonMatch[0]) as Record<string, unknown>;

    const factors = parsed.factors as Record<string, unknown> | undefined;

    return {
      score: clamp(Number(parsed.score) || 0),
      confidence: clamp(Number(parsed.confidence) || 0),
      recommendation: validateRecommendation(
        parsed.recommendation as string
      ),
      factors: {
        methodology_quality: clamp(
          Number(factors?.methodology_quality) || 0
        ),
        reputation: clamp(Number(factors?.reputation) || 0),
        novelty: clamp(Number(factors?.novelty) || 0),
        completeness: clamp(Number(factors?.completeness) || 0),
      },
      rationale: String(parsed.rationale || text),
    };
  } catch {
    return fallbackResult(text);
  }
}

function fallbackResult(text: string): ScreeningResult {
  return {
    score: 0,
    confidence: 0,
    recommendation: "CONDITIONAL",
    factors: {
      methodology_quality: 0,
      reputation: 0,
      novelty: 0,
      completeness: 0,
    },
    rationale: `Failed to parse structured response. Raw output:\n\n${text.slice(0, 1000)}`,
  };
}

function clamp(value: number): number {
  return Math.max(0, Math.min(1000, Math.round(value)));
}

function validateRecommendation(
  rec: string
): "APPROVE" | "CONDITIONAL" | "REJECT" {
  const upper = (rec || "").toUpperCase();
  if (upper === "APPROVE") return "APPROVE";
  if (upper === "REJECT") return "REJECT";
  return "CONDITIONAL";
}
