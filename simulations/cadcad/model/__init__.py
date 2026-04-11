"""
cadCAD economic simulation model for Regen Network mechanisms M012-M015.

This package implements the economic simulation specified in
docs/economics/economic-simulation-spec.md for parameter validation
of the Regen Economic Reboot.

Modules:
    state_variables -- Initial state vector for the simulation
    params          -- Complete parameter space with baseline, sweep, and stress configs
    policies        -- Policy functions P1-P7 (credit market, fees, mint/burn, rewards)
    state_updates   -- State update functions mapping policy outputs to next state
    config          -- cadCAD experiment configuration with partial state update blocks
"""
