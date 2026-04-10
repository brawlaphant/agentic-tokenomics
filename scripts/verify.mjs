#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

const repoRoot = process.cwd();

function requireFile(rel) {
  const p = path.join(repoRoot, rel);
  if (!fs.existsSync(p)) {
    console.error(`Missing required file: ${rel}`);
    process.exit(2);
  }
}

function run(cmd, args) {
  const res = spawnSync(cmd, args, { cwd: repoRoot, encoding: "utf8" });
  if (res.status !== 0) {
    console.error(res.stdout || "");
    console.error(res.stderr || "");
    process.exit(res.status ?? 3);
  }
}

function readJson(rel) {
  const p = path.join(repoRoot, rel);
  return JSON.parse(fs.readFileSync(p, "utf8"));
}

function listFilesRecursive(absDir) {
  const out = [];
  for (const entry of fs.readdirSync(absDir, { withFileTypes: true })) {
    const abs = path.join(absDir, entry.name);
    if (entry.isDirectory()) {
      out.push(...listFilesRecursive(abs));
    } else if (entry.isFile()) {
      out.push(abs);
    }
  }
  return out;
}

function assert(condition, message, exitCode) {
  if (!condition) {
    console.error(message);
    process.exit(exitCode);
  }
}

function validateSchema(rel, schema) {
  assert(schema && typeof schema === "object" && !Array.isArray(schema), `${rel}: schema must be a JSON object`, 5);
  assert(typeof schema.$schema === "string" && schema.$schema.length > 0, `${rel}: missing $schema`, 5);
  assert(schema.type === "object", `${rel}: top-level type must be object`, 5);
  assert(schema.properties && typeof schema.properties === "object" && !Array.isArray(schema.properties), `${rel}: missing properties object`, 5);

  if (schema.required !== undefined) {
    assert(Array.isArray(schema.required), `${rel}: required must be an array`, 5);
    const seen = new Set();
    for (const key of schema.required) {
      assert(typeof key === "string" && key.length > 0, `${rel}: required entries must be non-empty strings`, 5);
      assert(!seen.has(key), `${rel}: duplicate required entry '${key}'`, 5);
      seen.add(key);
      assert(Object.prototype.hasOwnProperty.call(schema.properties, key), `${rel}: required key '${key}' missing from properties`, 5);
    }
  }
}

// ---------------------------------------------------------------------------
// Mechanism registry — each entry declares its required files and self-tests.
// To add a new mechanism, append an entry here; no other changes needed.
// ---------------------------------------------------------------------------
const MECHANISMS = [
  {
    id: "m010",
    dir: "mechanisms/m010-reputation-signal",
    files: [
      "SPEC.md",
      "README.md",
      "schemas/m010_kpi.schema.json",
      "schemas/m010_signal.schema.json",
      "schemas/m010_challenge.schema.json",
      "datasets/schema.json",
      "datasets/fixtures/v0_sample.json",
      "datasets/fixtures/v0_challenge_sample.json",
      "datasets/fixtures/v0_challenge_escalated_sample.json",
      "datasets/fixtures/v0_challenge_edge_timing_sample.json",
      "datasets/fixtures/v0_challenge_invalid_resolution_sample.json",
      "datasets/fixtures/v0_challenge_invalid_outcome_sample.json",
      "reference-impl/m010_kpi.js",
      "reference-impl/m010_score.js",
      "reference-impl/test_vectors/vector_v0_sample.input.json",
      "reference-impl/test_vectors/vector_v0_sample.expected.json",
      "reference-impl/test_vectors/vector_v0_challenge.expected.json",
      "reference-impl/test_vectors/vector_v0_challenge_escalated.expected.json",
      "reference-impl/test_vectors/vector_v0_challenge_edge_timing.expected.json",
    ],
    kpiSchema: "schemas/m010_kpi.schema.json",
    selfTests: [],  // m010 uses dedicated verify scripts (below)
  },
  {
    id: "m012",
    dir: "mechanisms/m012-fixed-cap-dynamic-supply",
    files: [
      "SPEC.md",
      "README.md",
      "schemas/m012_kpi.schema.json",
      "schemas/m012_supply_state.schema.json",
      "schemas/m012_period_record.schema.json",
      "datasets/fixtures/v0_sample.json",
      "reference-impl/m012_supply.js",
      "reference-impl/m012_kpi.js",
    ],
    kpiSchema: "schemas/m012_kpi.schema.json",
    selfTests: ["reference-impl/m012_supply.js"],
  },
  {
    id: "m013",
    dir: "mechanisms/m013-value-based-fee-routing",
    files: [
      "SPEC.md",
      "README.md",
      "schemas/m013_kpi.schema.json",
      "schemas/m013_fee_event.schema.json",
      "schemas/m013_fee_config.schema.json",
      "datasets/fixtures/v0_sample.json",
    ],
    kpiSchema: "schemas/m013_kpi.schema.json",
    selfTests: ["reference-impl/m013_fee.js"],
  },
  {
    id: "m014",
    dir: "mechanisms/m014-authority-validator-governance",
    files: [
      "SPEC.md",
      "README.md",
      "schemas/m014_kpi.schema.json",
      "schemas/m014_performance.schema.json",
      "schemas/m014_validator.schema.json",
      "datasets/fixtures/v0_sample.json",
      "reference-impl/m014_kpi.js",
      "reference-impl/m014_score.js",
    ],
    kpiSchema: "schemas/m014_kpi.schema.json",
    selfTests: ["reference-impl/m014_score.js"],
  },
  {
    id: "m015",
    dir: "mechanisms/m015-contribution-weighted-rewards",
    files: [
      "SPEC.md",
      "schemas/m015_kpi.schema.json",
      "schemas/m015_activity_score.schema.json",
      "schemas/m015_stability_commitment.schema.json",
      "datasets/fixtures/v0_sample.json",
      "reference-impl/m015_score.js",
    ],
    kpiSchema: "schemas/m015_kpi.schema.json",
    selfTests: ["reference-impl/m015_score.js"],
  },
];

// ---------------------------------------------------------------------------
// Core repo files
// ---------------------------------------------------------------------------
requireFile("README.md");
requireFile("scripts/verify-m010-reference-impl.mjs");
requireFile("scripts/verify-m010-datasets.mjs");

// ---------------------------------------------------------------------------
// Per-mechanism: required files
// ---------------------------------------------------------------------------
for (const mech of MECHANISMS) {
  for (const file of mech.files) {
    requireFile(`${mech.dir}/${file}`);
  }
}

// ---------------------------------------------------------------------------
// Mechanism index check + m010-specific verification scripts
// ---------------------------------------------------------------------------
run("node", ["scripts/build-mechanism-index.mjs", "--check"]);
run("node", ["scripts/verify-m010-reference-impl.mjs"]);
run("node", ["scripts/verify-m010-datasets.mjs"]);

// ---------------------------------------------------------------------------
// Per-mechanism: KPI schema sanity (mechanism_id required) + self-tests
// ---------------------------------------------------------------------------
for (const mech of MECHANISMS) {
  if (mech.kpiSchema) {
    const schema = readJson(`${mech.dir}/${mech.kpiSchema}`);
    if (!schema.required || !schema.required.includes("mechanism_id")) {
      console.error(`${mech.id} KPI schema missing required fields.`);
      process.exit(4);
    }
  }
  for (const test of mech.selfTests) {
    run("node", [`${mech.dir}/${test}`]);
  }
}

// ---------------------------------------------------------------------------
// Schema sanity for all canonical .schema.json artifacts
// ---------------------------------------------------------------------------
const allFiles = listFilesRecursive(repoRoot);
const schemaFiles = allFiles
  .map((abs) => path.relative(repoRoot, abs))
  .filter((rel) => rel.endsWith(".schema.json"))
  .sort();

assert(schemaFiles.length > 0, "No .schema.json files found.", 4);
for (const rel of schemaFiles) {
  validateSchema(rel, readJson(rel));
}

// ---------------------------------------------------------------------------
// m010-specific schema invariants (challenge lifecycle, signal statuses)
// ---------------------------------------------------------------------------
const m010Kpi = readJson("mechanisms/m010-reputation-signal/schemas/m010_kpi.schema.json");
const challengeKpiRequired = m010Kpi.properties?.challenge_kpis?.required ?? [];
if (!challengeKpiRequired.includes("challenge_rate")) {
  console.error("KPI schema missing required challenge KPI fields.");
  process.exit(4);
}

const signalSchema = readJson("mechanisms/m010-reputation-signal/schemas/m010_signal.schema.json");
const signalStatus = signalSchema.properties?.status?.enum ?? [];
if (!signalStatus.includes("escalated")) {
  console.error("Signal schema missing escalated status.");
  process.exit(5);
}

const challengeSchema = readJson("mechanisms/m010-reputation-signal/schemas/m010_challenge.schema.json");
const challengeGuards = challengeSchema.allOf ?? [];
if (!Array.isArray(challengeGuards) || challengeGuards.length < 4) {
  console.error("Challenge schema missing lifecycle guard clauses.");
  process.exit(6);
}

console.log("agentic-tokenomics verify: PASS");
