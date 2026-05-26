# iBANK 🏦

**Rule-Based Money + Self-Lock Banking on Stellar**

A Soroban smart contract that auto-splits incoming funds into locked budget categories and enforces self-imposed spending rules that users cannot override until the lock period expires.

---

## Problem

Filipino freelancers receiving irregular USDC payments overspend within days because they lack enforced budgeting tools, leaving them unable to cover rent and essentials by month-end.

## Solution

iBANK uses Stellar Soroban smart contracts to:
1. Auto-split deposits into budget buckets (Savings, Food, Transport, Flexible)
2. Enforce daily spending limits on-chain
3. Lock rules via Self-Lock Mode—users literally cannot change them until expiry
4. Block unauthorized withdrawals from locked Savings buckets

---

## Timeline

Built for Stellar Soroban Bootcamp 2025 (SEA Cohort)

| Phase | Duration | Deliverable |
|-------|----------|-------------|
| Design | Day 1 | User flow + contract architecture |
| Contract | Days 2-3 | Core Soroban contract + tests |
| Frontend | Days 4-5 | React + Freighter integration |
| Demo | Day 6 | 2-minute video demo |

---

## Stellar Features Used

- **Soroban Smart Contracts** — All logic enforced on-chain
- **Stellar USDC/XLM** — Real money flow, not test tokens
- **Low Fees** — ~$0.0001 per transaction enables micro-spending
- **Freighter Wallet** — Seamless mobile-first UX
- **Trustlines** — Multi-asset support for USDC, PHP stablecoins

---

## Vision

iBANK becomes the default "salary account" for SEA freelancers—deposit once, spend confidently, build savings automatically. Future features include:

- AI spending coach analyzing bucket patterns
- Emergency unlock with 48-hour delay + penalty fee
- Group savings pools for families
- Merchant integrations for direct bucket payments

---

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (1.74+)
- [Soroban CLI](https://soroban.stellar.org/docs/getting-started/setup) (v21+)
- [Stellar Account](https://laboratory.stellar.org/) on Testnet

```bash
# Install Soroban CLI
cargo install --locked soroban-cli

# Add WASM target
rustup target add wasm32-unknown-unknown

#deployed contract
CCZRR3F6Y36X25DIKSAIHTEYOJ4C64VQNVCJXJTR3SAJXWI6XT5JBSQD
#proof
![alt text](<Screenshot .png>)