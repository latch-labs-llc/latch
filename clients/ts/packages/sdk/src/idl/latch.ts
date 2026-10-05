/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/latch.json`.
 */
export type Latch = {
  "address": "BT8tr7YXYLQPsLMq3qCipY1fBKyjT3gmGQB5amV2v5yv",
  "metadata": {
    "name": "latch",
    "version": "0.1.0",
    "spec": "0.1.0",
    "description": "Created with Anchor"
  },
  "instructions": [
    {
      "name": "approveMilestone",
      "discriminator": [
        145,
        85,
        92,
        60,
        50,
        130,
        219,
        106
      ],
      "accounts": [
        {
          "name": "party",
          "signer": true
        },
        {
          "name": "deal",
          "writable": true
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "index",
          "type": "u8"
        }
      ]
    },
    {
      "name": "cancelDraft",
      "discriminator": [
        234,
        193,
        92,
        79,
        29,
        206,
        188,
        175
      ],
      "accounts": [
        {
          "name": "party",
          "signer": true
        },
        {
          "name": "deal",
          "writable": true
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": []
    },
    {
      "name": "cancelSign",
      "discriminator": [
        29,
        16,
        171,
        97,
        132,
        108,
        17,
        148
      ],
      "accounts": [
        {
          "name": "party",
          "signer": true
        },
        {
          "name": "deal",
          "writable": true
        },
        {
          "name": "mint",
          "relations": [
            "deal"
          ]
        },
        {
          "name": "vault",
          "writable": true,
          "relations": [
            "deal"
          ]
        },
        {
          "name": "payerTokenAccount",
          "writable": true
        },
        {
          "name": "tokenProgram",
          "relations": [
            "deal"
          ]
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": []
    },
    {
      "name": "confirmReady",
      "discriminator": [
        151,
        150,
        182,
        46,
        101,
        22,
        246,
        43
      ],
      "accounts": [
        {
          "name": "party",
          "signer": true
        },
        {
          "name": "deal",
          "writable": true
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": []
    },
    {
      "name": "createDeal",
      "discriminator": [
        198,
        212,
        144,
        151,
        97,
        56,
        149,
        113
      ],
      "accounts": [
        {
          "name": "creator",
          "writable": true,
          "signer": true
        },
        {
          "name": "deal",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  97,
                  108
                ]
              },
              {
                "kind": "account",
                "path": "creator"
              },
              {
                "kind": "arg",
                "path": "params.dealId"
              }
            ]
          }
        },
        {
          "name": "mint"
        },
        {
          "name": "vault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "deal"
              },
              {
                "kind": "account",
                "path": "tokenProgram"
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ]
            }
          }
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "associatedTokenProgram",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "params",
          "type": {
            "defined": {
              "name": "createDealParams"
            }
          }
        }
      ]
    },
    {
      "name": "deposit",
      "discriminator": [
        242,
        35,
        198,
        137,
        82,
        225,
        242,
        182
      ],
      "accounts": [
        {
          "name": "payer",
          "signer": true
        },
        {
          "name": "deal",
          "writable": true
        },
        {
          "name": "mint",
          "relations": [
            "deal"
          ]
        },
        {
          "name": "vault",
          "writable": true,
          "relations": [
            "deal"
          ]
        },
        {
          "name": "payerTokenAccount",
          "writable": true
        },
        {
          "name": "tokenProgram",
          "relations": [
            "deal"
          ]
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "raiseDeadlock",
      "discriminator": [
        122,
        95,
        47,
        128,
        236,
        245,
        253,
        143
      ],
      "accounts": [
        {
          "name": "party",
          "signer": true
        },
        {
          "name": "deal",
          "writable": true
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": []
    },
    {
      "name": "recoveryExecute",
      "discriminator": [
        103,
        100,
        224,
        198,
        73,
        229,
        212,
        38
      ],
      "accounts": [
        {
          "name": "deal",
          "writable": true
        },
        {
          "name": "mint",
          "relations": [
            "deal"
          ]
        },
        {
          "name": "vault",
          "writable": true,
          "relations": [
            "deal"
          ]
        },
        {
          "name": "payerTokenAccount",
          "writable": true
        },
        {
          "name": "payeeTokenAccount",
          "writable": true
        },
        {
          "name": "tokenProgram",
          "relations": [
            "deal"
          ]
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": []
    },
    {
      "name": "recoverySign",
      "discriminator": [
        177,
        113,
        146,
        184,
        116,
        192,
        81,
        166
      ],
      "accounts": [
        {
          "name": "signer",
          "signer": true
        },
        {
          "name": "deal",
          "writable": true
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "amountToPayee",
          "type": "u64"
        }
      ]
    },
    {
      "name": "refundByPayee",
      "discriminator": [
        98,
        231,
        101,
        12,
        184,
        158,
        92,
        98
      ],
      "accounts": [
        {
          "name": "payee",
          "signer": true
        },
        {
          "name": "deal",
          "writable": true
        },
        {
          "name": "mint",
          "relations": [
            "deal"
          ]
        },
        {
          "name": "vault",
          "writable": true,
          "relations": [
            "deal"
          ]
        },
        {
          "name": "payerTokenAccount",
          "writable": true
        },
        {
          "name": "tokenProgram",
          "relations": [
            "deal"
          ]
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": []
    },
    {
      "name": "refundUnactivated",
      "discriminator": [
        77,
        80,
        6,
        249,
        39,
        198,
        33,
        102
      ],
      "accounts": [
        {
          "name": "cranker",
          "signer": true
        },
        {
          "name": "deal",
          "writable": true
        },
        {
          "name": "mint",
          "relations": [
            "deal"
          ]
        },
        {
          "name": "vault",
          "writable": true,
          "relations": [
            "deal"
          ]
        },
        {
          "name": "payerTokenAccount",
          "writable": true
        },
        {
          "name": "tokenProgram",
          "relations": [
            "deal"
          ]
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": []
    },
    {
      "name": "releaseMilestone",
      "discriminator": [
        56,
        2,
        199,
        164,
        184,
        108,
        167,
        222
      ],
      "accounts": [
        {
          "name": "deal",
          "writable": true
        },
        {
          "name": "mint",
          "relations": [
            "deal"
          ]
        },
        {
          "name": "vault",
          "writable": true,
          "relations": [
            "deal"
          ]
        },
        {
          "name": "payeeTokenAccount",
          "writable": true
        },
        {
          "name": "tokenProgram",
          "relations": [
            "deal"
          ]
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "index",
          "type": "u8"
        }
      ]
    },
    {
      "name": "resolutionSign",
      "discriminator": [
        163,
        229,
        220,
        139,
        228,
        47,
        64,
        220
      ],
      "accounts": [
        {
          "name": "signer",
          "signer": true
        },
        {
          "name": "deal",
          "writable": true
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "amountToPayee",
          "type": "u64"
        }
      ]
    },
    {
      "name": "resolve",
      "discriminator": [
        246,
        150,
        236,
        206,
        108,
        63,
        58,
        10
      ],
      "accounts": [
        {
          "name": "deal",
          "writable": true
        },
        {
          "name": "mint",
          "relations": [
            "deal"
          ]
        },
        {
          "name": "vault",
          "writable": true,
          "relations": [
            "deal"
          ]
        },
        {
          "name": "payerTokenAccount",
          "writable": true
        },
        {
          "name": "payeeTokenAccount",
          "writable": true
        },
        {
          "name": "tokenProgram",
          "relations": [
            "deal"
          ]
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": []
    },
    {
      "name": "signTerms",
      "discriminator": [
        226,
        42,
        174,
        143,
        144,
        159,
        139,
        1
      ],
      "accounts": [
        {
          "name": "party",
          "signer": true
        },
        {
          "name": "deal",
          "writable": true
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": [
        {
          "name": "consentTrueDeadlock",
          "type": "bool"
        }
      ]
    },
    {
      "name": "withdrawDeadlock",
      "discriminator": [
        173,
        19,
        196,
        62,
        64,
        15,
        170,
        69
      ],
      "accounts": [
        {
          "name": "party",
          "signer": true
        },
        {
          "name": "deal",
          "writable": true
        },
        {
          "name": "eventAuthority"
        },
        {
          "name": "program"
        }
      ],
      "args": []
    }
  ],
  "accounts": [
    {
      "name": "deal",
      "discriminator": [
        125,
        223,
        160,
        234,
        71,
        162,
        182,
        219
      ]
    }
  ],
  "events": [
    {
      "name": "activationLapsed",
      "discriminator": [
        97,
        227,
        102,
        255,
        29,
        12,
        220,
        238
      ]
    },
    {
      "name": "cancelSigned",
      "discriminator": [
        146,
        67,
        52,
        179,
        173,
        19,
        155,
        113
      ]
    },
    {
      "name": "deadlockRaised",
      "discriminator": [
        125,
        252,
        195,
        145,
        42,
        95,
        54,
        121
      ]
    },
    {
      "name": "deadlockWithdrawn",
      "discriminator": [
        124,
        43,
        79,
        171,
        85,
        162,
        155,
        96
      ]
    },
    {
      "name": "dealCancelled",
      "discriminator": [
        229,
        189,
        86,
        176,
        134,
        151,
        43,
        152
      ]
    },
    {
      "name": "dealCreated",
      "discriminator": [
        27,
        18,
        50,
        52,
        104,
        175,
        46,
        101
      ]
    },
    {
      "name": "dealResolved",
      "discriminator": [
        248,
        246,
        197,
        42,
        190,
        103,
        95,
        56
      ]
    },
    {
      "name": "depositReceived",
      "discriminator": [
        9,
        208,
        152,
        63,
        64,
        32,
        185,
        118
      ]
    },
    {
      "name": "milestoneApproved",
      "discriminator": [
        40,
        109,
        159,
        144,
        169,
        230,
        35,
        229
      ]
    },
    {
      "name": "milestoneReleased",
      "discriminator": [
        49,
        225,
        91,
        223,
        34,
        165,
        109,
        181
      ]
    },
    {
      "name": "partyReady",
      "discriminator": [
        93,
        240,
        251,
        1,
        126,
        229,
        120,
        129
      ]
    },
    {
      "name": "payeeRefunded",
      "discriminator": [
        232,
        6,
        34,
        254,
        145,
        67,
        190,
        245
      ]
    },
    {
      "name": "recoverySigned",
      "discriminator": [
        76,
        168,
        96,
        58,
        177,
        160,
        210,
        41
      ]
    },
    {
      "name": "resolutionSigned",
      "discriminator": [
        12,
        125,
        221,
        248,
        242,
        103,
        239,
        27
      ]
    },
    {
      "name": "termsSigned",
      "discriminator": [
        88,
        7,
        99,
        39,
        64,
        200,
        154,
        151
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "invalidParties",
      "msg": "Invalid party configuration"
    },
    {
      "code": 6001,
      "name": "duplicateParty",
      "msg": "Duplicate party pubkey"
    },
    {
      "code": 6002,
      "name": "creatorNotParty",
      "msg": "Creator must be one of the parties"
    },
    {
      "code": 6003,
      "name": "invalidMilestones",
      "msg": "Invalid milestone configuration"
    },
    {
      "code": 6004,
      "name": "milestoneSumMismatch",
      "msg": "Milestone amounts must sum to the total amount"
    },
    {
      "code": 6005,
      "name": "invalidThreshold",
      "msg": "Invalid approval threshold"
    },
    {
      "code": 6006,
      "name": "invalidDeadlockParams",
      "msg": "Invalid deadlock rule parameters"
    },
    {
      "code": 6007,
      "name": "invalidRecoveryConfig",
      "msg": "Invalid recovery configuration"
    },
    {
      "code": 6008,
      "name": "notAParty",
      "msg": "Signer is not a party to this deal"
    },
    {
      "code": 6009,
      "name": "notARecoverySigner",
      "msg": "Signer is not a recovery signer for this deal"
    },
    {
      "code": 6010,
      "name": "invalidState",
      "msg": "Instruction not valid in the deal's current state"
    },
    {
      "code": 6011,
      "name": "alreadySigned",
      "msg": "Party has already signed"
    },
    {
      "code": 6012,
      "name": "trueDeadlockConsentRequired",
      "msg": "True-deadlock rule requires explicit consent from every party"
    },
    {
      "code": 6013,
      "name": "onlyPayerMayDeposit",
      "msg": "Only the payer may deposit"
    },
    {
      "code": 6014,
      "name": "zeroDeposit",
      "msg": "Deposit amount must be greater than zero"
    },
    {
      "code": 6015,
      "name": "overFunded",
      "msg": "Deposit exceeds the deal's total amount"
    },
    {
      "code": 6016,
      "name": "milestoneOutOfRange",
      "msg": "Milestone index out of range"
    },
    {
      "code": 6017,
      "name": "milestoneAlreadyReleased",
      "msg": "Milestone already released"
    },
    {
      "code": 6018,
      "name": "milestonesOutOfOrder",
      "msg": "Previous milestones must be released first"
    },
    {
      "code": 6019,
      "name": "insufficientApprovals",
      "msg": "Not enough approvals to release this milestone"
    },
    {
      "code": 6020,
      "name": "resolutionConditionsNotMet",
      "msg": "Resolution conditions are not met"
    },
    {
      "code": 6021,
      "name": "timeoutNotElapsed",
      "msg": "Timeout has not elapsed"
    },
    {
      "code": 6022,
      "name": "noTimeoutPath",
      "msg": "This deadlock rule never resolves by timeout"
    },
    {
      "code": 6023,
      "name": "payoutExceedsVault",
      "msg": "Proposed payout exceeds the vault balance"
    },
    {
      "code": 6024,
      "name": "insufficientRecoverySignatures",
      "msg": "Not enough recovery signatures"
    },
    {
      "code": 6025,
      "name": "noActiveProposal",
      "msg": "No active proposal"
    },
    {
      "code": 6026,
      "name": "tokenAccountMismatch",
      "msg": "Token account does not match the deal"
    },
    {
      "code": 6027,
      "name": "mintMismatch",
      "msg": "Mint does not match the deal"
    },
    {
      "code": 6028,
      "name": "tokenProgramMismatch",
      "msg": "Token program does not match the deal"
    },
    {
      "code": 6029,
      "name": "nonTransferableMint",
      "msg": "Mint is non-transferable; escrow is impossible"
    },
    {
      "code": 6030,
      "name": "defaultFrozenMint",
      "msg": "Mint's default account state is frozen; escrow vault would be unusable"
    },
    {
      "code": 6031,
      "name": "activeTransferHook",
      "msg": "Mint has an active transfer hook program; not supported"
    },
    {
      "code": 6032,
      "name": "unknownMintExtension",
      "msg": "Mint carries an unknown Token-2022 extension; rejected by default"
    },
    {
      "code": 6033,
      "name": "riskFlagsNotAccepted",
      "msg": "Parties have not accepted this mint's risk flags"
    },
    {
      "code": 6034,
      "name": "timeoutPausedByDispute",
      "msg": "Timeout clock is paused while a dispute is open"
    },
    {
      "code": 6035,
      "name": "onlyRaiserMayWithdraw",
      "msg": "Only the party who raised the deadlock may withdraw it"
    },
    {
      "code": 6036,
      "name": "invalidTimerMode",
      "msg": "This timer mode is not valid for the chosen deadlock rule"
    },
    {
      "code": 6037,
      "name": "recoveryDelayNotElapsed",
      "msg": "Recovery notice delay has not elapsed"
    },
    {
      "code": 6038,
      "name": "overflow",
      "msg": "Arithmetic overflow"
    },
    {
      "code": 6039,
      "name": "activationWindowOpen",
      "msg": "Activation window has not lapsed"
    },
    {
      "code": 6040,
      "name": "onlyPayeeMayRefund",
      "msg": "Only the payee may refund the payer"
    }
  ],
  "types": [
    {
      "name": "activationLapsed",
      "docs": [
        "A funded deal never became Active within the activation window, so the",
        "deposit was returned to the payer. Distinct from `DealCancelled` so the",
        "record shows the refund happened by lapse, not by mutual consent."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "deal",
            "type": "pubkey"
          },
          {
            "name": "seq",
            "type": "u64"
          },
          {
            "name": "refundedToPayer",
            "type": "u64"
          },
          {
            "name": "fundedAt",
            "type": "i64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "cancelSigned",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "deal",
            "type": "pubkey"
          },
          {
            "name": "seq",
            "type": "u64"
          },
          {
            "name": "party",
            "type": "pubkey"
          },
          {
            "name": "allSigned",
            "type": "bool"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "createDealParams",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "dealId",
            "type": "u64"
          },
          {
            "name": "parties",
            "type": {
              "vec": "pubkey"
            }
          },
          {
            "name": "payerIdx",
            "type": "u8"
          },
          {
            "name": "payeeIdx",
            "type": "u8"
          },
          {
            "name": "approvalThreshold",
            "type": "u8"
          },
          {
            "name": "termsHash",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "priceRiskTerm",
            "type": "u8"
          },
          {
            "name": "totalAmount",
            "type": "u64"
          },
          {
            "name": "milestoneAmounts",
            "type": {
              "vec": "u64"
            }
          },
          {
            "name": "deadlockRule",
            "type": {
              "defined": {
                "name": "deadlockRule"
              }
            }
          },
          {
            "name": "timerMode",
            "type": {
              "defined": {
                "name": "timerMode"
              }
            }
          },
          {
            "name": "timeoutSecs",
            "type": "i64"
          },
          {
            "name": "splitBps",
            "type": "u16"
          },
          {
            "name": "tieBreaker",
            "type": "pubkey"
          },
          {
            "name": "recoverySigners",
            "type": {
              "vec": "pubkey"
            }
          },
          {
            "name": "recoveryThreshold",
            "type": "u8"
          },
          {
            "name": "recoveryDelaySecs",
            "type": "i64"
          },
          {
            "name": "acceptedRiskFlags",
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "deadlockRaised",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "deal",
            "type": "pubkey"
          },
          {
            "name": "seq",
            "type": "u64"
          },
          {
            "name": "raisedBy",
            "type": "pubkey"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "deadlockRule",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "timeoutRelease"
          },
          {
            "name": "timeoutRefund"
          },
          {
            "name": "autoSplit"
          },
          {
            "name": "tieBreaker"
          },
          {
            "name": "longSunset"
          },
          {
            "name": "trueDeadlock"
          }
        ]
      }
    },
    {
      "name": "deadlockWithdrawn",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "deal",
            "type": "pubkey"
          },
          {
            "name": "seq",
            "type": "u64"
          },
          {
            "name": "withdrawnBy",
            "type": "pubkey"
          },
          {
            "name": "elapsedSecs",
            "docs": [
              "Active-clock seconds already elapsed (FromActivation mode)."
            ],
            "type": "i64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "deal",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "version",
            "type": "u8"
          },
          {
            "name": "bump",
            "type": "u8"
          },
          {
            "name": "dealId",
            "type": "u64"
          },
          {
            "name": "creator",
            "type": "pubkey"
          },
          {
            "name": "state",
            "type": {
              "defined": {
                "name": "dealState"
              }
            }
          },
          {
            "name": "numParties",
            "type": "u8"
          },
          {
            "name": "parties",
            "type": {
              "array": [
                "pubkey",
                8
              ]
            }
          },
          {
            "name": "payerIdx",
            "type": "u8"
          },
          {
            "name": "payeeIdx",
            "type": "u8"
          },
          {
            "name": "approvalThreshold",
            "docs": [
              "M-of-N approvals required to release a milestone."
            ],
            "type": "u8"
          },
          {
            "name": "signed",
            "type": "u8"
          },
          {
            "name": "ready",
            "type": "u8"
          },
          {
            "name": "deadlockConsent",
            "type": "u8"
          },
          {
            "name": "cancelApprovals",
            "type": "u8"
          },
          {
            "name": "partiesSignedAt",
            "type": {
              "array": [
                "i64",
                8
              ]
            }
          },
          {
            "name": "termsHash",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "priceRiskTerm",
            "type": "u8"
          },
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "tokenProgram",
            "type": "pubkey"
          },
          {
            "name": "vault",
            "type": "pubkey"
          },
          {
            "name": "totalAmount",
            "type": "u64"
          },
          {
            "name": "deposited",
            "docs": [
              "Credited by vault balance delta, so transfer-fee mints can't corrupt accounting."
            ],
            "type": "u64"
          },
          {
            "name": "releasedTotal",
            "type": "u64"
          },
          {
            "name": "riskFlags",
            "type": "u16"
          },
          {
            "name": "acceptedRiskFlags",
            "type": "u16"
          },
          {
            "name": "numMilestones",
            "type": "u8"
          },
          {
            "name": "milestones",
            "type": {
              "array": [
                {
                  "defined": {
                    "name": "milestone"
                  }
                },
                16
              ]
            }
          },
          {
            "name": "deadlockRule",
            "type": {
              "defined": {
                "name": "deadlockRule"
              }
            }
          },
          {
            "name": "timerMode",
            "type": {
              "defined": {
                "name": "timerMode"
              }
            }
          },
          {
            "name": "timeoutSecs",
            "type": "i64"
          },
          {
            "name": "splitBps",
            "type": "u16"
          },
          {
            "name": "tieBreaker",
            "type": "pubkey"
          },
          {
            "name": "deadlockRaisedAt",
            "type": "i64"
          },
          {
            "name": "deadlockRaisedBy",
            "type": "pubkey"
          },
          {
            "name": "elapsedAtPause",
            "docs": [
              "FromActivation accounting: seconds of Active-state clock accumulated",
              "before the current pause, and when the clock last (re)started."
            ],
            "type": "i64"
          },
          {
            "name": "lastResumedAt",
            "type": "i64"
          },
          {
            "name": "proposalActive",
            "type": "bool"
          },
          {
            "name": "proposedToPayee",
            "type": "u64"
          },
          {
            "name": "resolutionApprovals",
            "type": "u8"
          },
          {
            "name": "tieBreakerDecided",
            "type": "bool"
          },
          {
            "name": "numRecovery",
            "type": "u8"
          },
          {
            "name": "recoveryThreshold",
            "type": "u8"
          },
          {
            "name": "recoverySigners",
            "type": {
              "array": [
                "pubkey",
                3
              ]
            }
          },
          {
            "name": "recoveryProposalActive",
            "type": "bool"
          },
          {
            "name": "recoveryProposedToPayee",
            "type": "u64"
          },
          {
            "name": "recoveryApprovals",
            "type": "u8"
          },
          {
            "name": "recoveryDelaySecs",
            "docs": [
              "Notice delay between \"threshold of signatures reached\" and executable."
            ],
            "type": "i64"
          },
          {
            "name": "recoveryReadyAt",
            "docs": [
              "When the current proposal first met the threshold (0 = not yet)."
            ],
            "type": "i64"
          },
          {
            "name": "createdAt",
            "type": "i64"
          },
          {
            "name": "signedAt",
            "type": "i64"
          },
          {
            "name": "fundedAt",
            "type": "i64"
          },
          {
            "name": "activatedAt",
            "type": "i64"
          },
          {
            "name": "eventSeq",
            "docs": [
              "Monotonic event sequence, one per emitted event."
            ],
            "type": "u64"
          },
          {
            "name": "tieBreakerAmount",
            "docs": [
              "The tie-breaker's ruled payout (meaningful only while",
              "`tie_breaker_decided`). Stored apart from `proposed_to_payee` so a",
              "party counter-proposal can never overwrite or erase the ruling.",
              "Carved from the front of `_reserved` (zero in pre-existing accounts),",
              "so the account layout and size are unchanged."
            ],
            "type": "u64"
          },
          {
            "name": "reserved",
            "docs": [
              "Reserved for future use (ZK phase: per-party ElGamal keys, etc.)."
            ],
            "type": {
              "array": [
                "u8",
                56
              ]
            }
          }
        ]
      }
    },
    {
      "name": "dealCancelled",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "deal",
            "type": "pubkey"
          },
          {
            "name": "seq",
            "type": "u64"
          },
          {
            "name": "refundedToPayer",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "dealCreated",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "deal",
            "type": "pubkey"
          },
          {
            "name": "seq",
            "type": "u64"
          },
          {
            "name": "dealId",
            "type": "u64"
          },
          {
            "name": "creator",
            "type": "pubkey"
          },
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "totalAmount",
            "type": "u64"
          },
          {
            "name": "numParties",
            "type": "u8"
          },
          {
            "name": "numMilestones",
            "type": "u8"
          },
          {
            "name": "termsHash",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "riskFlags",
            "type": "u16"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "dealResolved",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "deal",
            "type": "pubkey"
          },
          {
            "name": "seq",
            "type": "u64"
          },
          {
            "name": "path",
            "type": {
              "defined": {
                "name": "resolutionPath"
              }
            }
          },
          {
            "name": "paidToPayee",
            "type": "u64"
          },
          {
            "name": "refundedToPayer",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "dealState",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "draft"
          },
          {
            "name": "signed"
          },
          {
            "name": "funded"
          },
          {
            "name": "active"
          },
          {
            "name": "deadlocked"
          },
          {
            "name": "completed"
          },
          {
            "name": "cancelled"
          }
        ]
      }
    },
    {
      "name": "depositReceived",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "deal",
            "type": "pubkey"
          },
          {
            "name": "seq",
            "type": "u64"
          },
          {
            "name": "from",
            "type": "pubkey"
          },
          {
            "name": "amountCredited",
            "type": "u64"
          },
          {
            "name": "depositedTotal",
            "type": "u64"
          },
          {
            "name": "fullyFunded",
            "type": "bool"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "milestone",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "approvals",
            "docs": [
              "Bitmap: bit i set = party i approved this milestone."
            ],
            "type": "u8"
          },
          {
            "name": "released",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "milestoneApproved",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "deal",
            "type": "pubkey"
          },
          {
            "name": "seq",
            "type": "u64"
          },
          {
            "name": "milestoneIndex",
            "type": "u8"
          },
          {
            "name": "party",
            "type": "pubkey"
          },
          {
            "name": "approvals",
            "type": "u8"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "milestoneReleased",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "deal",
            "type": "pubkey"
          },
          {
            "name": "seq",
            "type": "u64"
          },
          {
            "name": "milestoneIndex",
            "type": "u8"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "releasedTotal",
            "type": "u64"
          },
          {
            "name": "completed",
            "type": "bool"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "partyReady",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "deal",
            "type": "pubkey"
          },
          {
            "name": "seq",
            "type": "u64"
          },
          {
            "name": "party",
            "type": "pubkey"
          },
          {
            "name": "allReady",
            "type": "bool"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "payeeRefunded",
      "docs": [
        "The payee voluntarily returned everything left in the vault to the payer."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "deal",
            "type": "pubkey"
          },
          {
            "name": "seq",
            "type": "u64"
          },
          {
            "name": "payee",
            "type": "pubkey"
          },
          {
            "name": "refundedToPayer",
            "type": "u64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "recoverySigned",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "deal",
            "type": "pubkey"
          },
          {
            "name": "seq",
            "type": "u64"
          },
          {
            "name": "signer",
            "type": "pubkey"
          },
          {
            "name": "proposedToPayee",
            "type": "u64"
          },
          {
            "name": "signatures",
            "type": "u8"
          },
          {
            "name": "threshold",
            "type": "u8"
          },
          {
            "name": "executableAt",
            "docs": [
              "When this proposal becomes executable (0 = threshold not yet met).",
              "On-chain notice: parties can see a pending recovery and object or settle."
            ],
            "type": "i64"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "resolutionPath",
      "docs": [
        "How a deadlock ended — carried in the DealResolved event."
      ],
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "mutual"
          },
          {
            "name": "tieBreaker"
          },
          {
            "name": "timeout"
          },
          {
            "name": "recovery"
          }
        ]
      }
    },
    {
      "name": "resolutionSigned",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "deal",
            "type": "pubkey"
          },
          {
            "name": "seq",
            "type": "u64"
          },
          {
            "name": "signer",
            "type": "pubkey"
          },
          {
            "name": "proposedToPayee",
            "type": "u64"
          },
          {
            "name": "isTieBreaker",
            "type": "bool"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "termsSigned",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "deal",
            "type": "pubkey"
          },
          {
            "name": "seq",
            "type": "u64"
          },
          {
            "name": "party",
            "type": "pubkey"
          },
          {
            "name": "partyIndex",
            "type": "u8"
          },
          {
            "name": "termsHash",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "allSigned",
            "type": "bool"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "timerMode",
      "docs": [
        "When the timeout clock for time-based deadlock rules runs."
      ],
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "fromDeadlock"
          },
          {
            "name": "fromActivation"
          }
        ]
      }
    }
  ],
  "constants": [
    {
      "name": "activationWindowSecs",
      "docs": [
        "How long a fully funded deal may wait for every party to confirm ready",
        "(3 days). If it has not become Active by then, anyone may return the whole",
        "vault to the payer: without this, a counterparty who never confirms could",
        "hold the payer's deposit indefinitely, since every other exit from Funded",
        "needs that counterparty's signature."
      ],
      "type": "i64",
      "value": "259200"
    },
    {
      "name": "dealSeed",
      "type": "bytes",
      "value": "[100, 101, 97, 108]"
    }
  ]
};
