# xArcade game template

Build a game for xArcade and have it listed on the platform. This repository is a
**complete, working game contract** ("Lucky Number": bet on one face of a die) with every part you would
otherwise get wrong already in place. You change the rules; the plumbing stays.

> **How the whole process works**
> 1. Fork this repository and build your game (this page).
> 2. Add the xArcade GitHub account as an **admin** of your fork, so we can review it.
> 3. Post your game on the **Game Lab** page of xArcade, with the link to your repository.
> 4. We **audit** the contract. You see the stage on your post and get a notification at each change.
> 5. Once approved, you deploy it, we list it, and you fund its bankroll. Terms (fees, limits) are agreed with you first.
>
> Listing is never guaranteed, and nothing here is a promise of earnings.

## The one idea you must understand

**Your contract holds rules, never money.** Every token a player bets sits in the xArcade **hub**. The hub takes
the bet, asks your game what it may have to pay, and later asks your game who won. Then the hub pays. That is why:

* every endpoint of your game can only be called **by the hub** (`require_hub`), never by a player;
* your game never receives or sends tokens, and must not try to.

```
player ──bet──▶  HUB  ──joinGame──▶  your game      "what is the most you could pay for this?"
                 HUB  ──resolve───▶  your game      "who won, and how much?"   ──▶ HUB pays
```

## Quick start

You need [Rust](https://www.rust-lang.org/tools/install) and the MultiversX `sc-meta` tool
(`cargo install multiversx-sc-meta --locked`).

```bash
git clone <your fork>
cd xarcade-game-template
sc-meta all build     # builds the contract into output/
cargo test            # runs the tests against the build (build first, every time you change the code)
```

## What to change, and what not to

Open `src/play.rs`. Everything you may change is marked **`YOUR GAME`**.

| Change | Keep exactly as it is |
|---|---|
| The numbers at the top of `play.rs` (`PAYOUT_MULTIPLIER`, `DIE_FACES`) | The names, arguments and return types of the four endpoints |
| The rules inside `join_game` (what `data` means, what you accept) | `require_hub()` at the start of every endpoint |
| The rules inside `resolve` (who wins, how much) | The order of the fields in `GameResolveResult` and the `GameMode` enum |
| The `Game` struct in `types.rs` (what you remember per game) | Removing the game in `resolve` and `cancel_game` |

## What the hub asks of your game

Four endpoints. The hub calls them with exactly these arguments, in this order:

| Endpoint | Arguments | Returns | What it must do |
|---|---|---|---|
| `joinGame` | `game_id`, `is_new`, `player`, `token`, `token_nonce`, `wager`, `mode`, `data` | a list of payments | Check the bet is playable, remember the game, and **return the most it could ever have to pay out** for this player. The hub locks exactly that much of your bankroll. |
| `resolve` | `game_id` | `GameResolveResult` | Decide the result and **delete the game**. PvE: `winner` is the player, or the **zero address** when the house wins. `payout` is what the player gets. |
| `leaveGame` | `game_id` | nothing | A PvP lobby creator leaves. PvE-only games refuse it. |
| `cancelGame` | `game_id` | nothing | A player took their stake back because the game was never settled. **Delete the game. Never fail.** |

`data` is whatever the player sent with their bet (a byte, a number, a choice). Your game decides what it means
and **validates it**. The hub has already checked the bet is inside the limits it set for your game.

### Rules the hub enforces on you

The hub does not trust your game, and checks these. A game that breaks them fails on chain.

* **Payout ≤ reservation.** A payout above what `joinGame` returned is refused ("Payout exceeds reservation").
  In the template the payout is exactly the reservation, so it can never be above it.
* **At most 16 reservations** per join.
* **Only the hub may call you**, and your code **must not be upgradeable**: the hub refuses to list a contract
  whose code could be swapped after the review.
* **The house has no address.** A house win is the zero address as `winner`.
* **The hub pays winners and keeps its fee** (set by the hub's owner, at most 10%). Your payout is the gross
  figure: do not subtract the hub's fee yourself.
* A game that can't be settled is not lost: after a delay the player can take their stake back from the hub.
  But a game whose `resolve` fails for a valid game is a broken game, and the audit will say so.

## Writing safe rules

* **Reserve the maximum.** If a win pays 5x the wager, return `wager × 5`. If you return less and pay more, the
  hub refuses the payout and the game is stuck. If you return more, you lock more of your bankroll than needed.
* **Randomness.** The template uses the block's random seed mixed with the game id, taken when the game is
  resolved. Use only what a player cannot choose or predict: never a number the player sent, their address, or
  the block timestamp. Know the limit: the platform's resolver chooses *when* a game settles, so it chooses which
  block's seed is used. Your code cannot remove that, so do not build a game that pays enormous jackpots on it.
* **Keep `resolve` cheap and certain to succeed.** No loops over everything ever played, no calls to other
  contracts, no storage that grows without end. If `resolve` can run out of gas or revert, a player's game gets
  stuck.
* **Clean up.** Delete the game in `resolve` and in `cancel_game`. A game left behind is a game the hub can ask
  about again.
* **Validate everything you read from `data`**, and reject what you cannot play (the template refuses NFTs,
  zero bets, bad picks and PvP).
* **Your house must be able to pay.** You fund the game's bankroll and you are the house: a bankroll that cannot
  cover the reservations means bets are refused.

## Tests

`tests/template_test.rs` runs the contract with a plain account standing in for the hub. The tests named
`control_hub_rules_*` check what the hub requires of every game: **keep them passing**, whatever your rules.
Then add tests for your own rules (what wins, what loses, what you refuse).

The platform's own tests also run this template against the **real hub** (it plays full games, settles wins and
losses, and takes a stake back). Yours will be run the same way during the review.

## Deploying to devnet

1. `sc-meta all build`.
2. Deploy `output/xarcade-game-template.wasm` to devnet with one argument: the **xArcade hub address**
   (the xArcade team publishes the current one next to this template). Deploy it **without** the upgradeable flag; check the code metadata of
   the deployed contract, the hub refuses an upgradeable one.
3. Post your game on **Game Lab** with the link to your repository.
4. After approval we list the contract on the hub and set its bet limits, and you fund the bankroll.

## Going further

* **PvP games** (two players, the winner takes both stakes): see `xarcade-sc-coinflip` in the platform repository.
  Joining an existing game uses `is_new = false`, nothing is reserved (the hub keeps the stakes), and
  `resolve` returns both a `winner` and a `loser`.
* **Prize pools** (a game that pays out of a fixed stock of prizes, like a loot box): see `xarcade-sc-lootbox`.

## Before you submit: checklist

- [ ] `sc-meta all build` and `cargo test` both pass.
- [ ] Every endpoint starts with `require_hub()`.
- [ ] `joinGame` returns the most you could pay, and `resolve` never pays more.
- [ ] A house win returns the zero address as winner.
- [ ] `resolve` and `cancelGame` delete the game, and `cancelGame` never fails.
- [ ] The randomness uses only what a player cannot influence.
- [ ] You wrote tests for your own rules.
- [ ] The xArcade GitHub account is an admin of your repository.
- [ ] Your Game Lab post explains the rules in plain words.
