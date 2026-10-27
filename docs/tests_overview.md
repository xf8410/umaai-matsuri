# 测试一览（骏大祭 / matsuri）

> **前期验证以对话核对为主**：机制是否成立由用户按 [`plan/intake.md`](plan/intake.md) 的确认清单逐条核对，
> 自动化测试随业务落地逐步补齐，不用测试代替机制确认。
> 分工：单元（内联，私有细节 / 纯函数，`assert`）/ 机制集成（`tests/`，打印 + `Result`）/
> 冒烟（`cli/tests/`）/ 基线（`baselines/`，自动判定）。
> 测试 cwd 在各 crate 目录；路径统一经 `matsuri_core::testing` 取仓根。

## 1. 分 crate

| crate | 测试面 | 现状 |
|---|---|---|
| `matsuri-core` | 内联单测 + `tests/mechanics.rs`（机制集成） | 数据层 11 例 + 配置 / 阶段 / 训练员已有；机制集成待补（等 §4 缺口收口） |
| `matsuri-sim` | `tests/baseline.rs`（固定种子基线） | 待补 |
| `matsuri-cli` | `tests/smoke.rs`（一局冒烟 + 可复现） | 待补 |
| `matsuri-ai` / `matsuri-tools` | 骨架，暂无 | — |

## 2. 单元测试（内联）

| 文件 | 用例 | 守门内容 |
|---|---|---|
| `core/src/data.rs` | `load_repo_data` | 仓内 `data/scenario_matsuri.json` 按 schema 读通，逐表核行数（担当 5 / 演目效果 66 / 排程 3 / 窗口 6 / 教诲 10 / 教诲效果 153 / 転成 9 / 発表会 20 / 本番 4 / 出行 10 / 稽古基础值 5），并核对 turn 已转 **0 基**（`23 / 47 / 67`、`2 / 11 / 23 / 35 / 47 / 59`） |
| `core/src/data.rs` | `effect_values_land_in_different_columns` | 读取纪律：同为 `effect_type = 13`，教诲 `101` 只填第 4 列、`601` 同填第 2 与第 4 列 → 禁止按列硬编码（intake §4 落库口径 3，表见 §3.4） |
| `core/src/data.rs` | `reject_unknown_field` | `deny_unknown_fields`：资产多一个键即报错 |
| `core/src/data.rs` | `reject_missing_act_level` | `act_effect` 缺任一 Lv1–10 档位即报错 |
| `core/src/data.rs` | `reject_unobserved_result_state` | 発表会 `result_state = 3` 未观察到 → 载入期越界报错，不静默兜底 |
| `core/src/data.rs` | `reject_unsorted_schedule` | `schedule.id` 连续、turn 严格升序且在 `0..=MAX_TURN` 内 |
| `core/src/data.rs` | `reject_dangling_advice_id` | `advice_effect` / `fusion_effect` 的 `advice_id` 必须在 `advice` 表内（跨表引用完整性） |
| `core/src/data.rs` | `reject_unknown_act_command` | `act_training_basic` 的命令必须在担当表 `command_id` 内（防串表） |
| `core/src/data.rs` | `reject_zero_gauge_max` | `act_gauge_max = 0` 会让演目稽古永不点亮 → 报错 |
| `core/src/data.rs` | `reject_wrong_training_count` / `reject_bad_row_width` | 训练基础值表形状：5 个训练、每档 7 值 |
| `core/src/config.rs` | `parse_base_ok` / `overlay_replaces_list` / `unknown_key_rejected` / `out_of_range_index_rejected` | 默认层 / 覆盖层 / 未知键 / 下标 |
| `core/src/game/stage.rs` | `next_chain` / `stage_ids_are_stable` | 阶段链 / 阶段号稳定 |
| `core/src/trainer.rs` | `preferred_training_wraps` | 样例训练员轮转 |

> 数据层单测只守「读得到 + 校验得住 + 口径不漂移」；**机制结算用例在对应机制收口后才写**
> （不用测试代替机制确认，也不写按猜测实现的断言）。

## 3. 机制集成（`core/tests/mechanics.rs`）

> 每条机制至少一个用例；机制未确认前不写「按猜测实现」的测试。

| 用例 | 守门内容 | 对应机制文档 |
|---|---|---|
| `<待补>` | 演目 / gauge / 稽古 / 教え / 合宿 / 発表会 | `mechanism.md` §1–§6 |

## 4. 冒烟与基线

| 项 | 内容 | 判定 |
|---|---|---|
| 冒烟 | 固定种子跑满回合、两次逐位一致 | `Result` / 非零退出 |
| 基线 | 一基线一 JSON（`input` 自持、`expect` 逐项容差、`meta` 不含路径） | 自动比对，失败返回 `Err` |

- 重录命令：`<cargo ... baseline --update>`（S9 落地）
- 数值占位替换后必须重录，并在 `glossary.md` 记影响面。
- **工具链**：引擎清单含 `umasim = { workspace = true, default-features = false }`，
  在 `cargo 1.98.1` 下直接拒绝装载（`default-features` 不允许覆盖 workspace 值）→
  本地一律按引擎 `rust-toolchain.toml` 的 **1.99.0** 跑
  `cargo fmt --all --check` / `cargo clippy --workspace --all-targets --offline -- -D warnings` /
  `cargo test --workspace --release --offline`；CI 也钉同一版本（`dtolnay/rust-toolchain@1.99.0`，见 §6），
  避免 stable 漂移导致 rustfmt 细节不一致。
- **2026-10-27 实跑记录**（工具链 **1.99.0**：rustc 1.99.0 / cargo 1.99.0 / rustfmt 1.10.0-stable / clippy 0.1.99）：
  `cargo +1.99.0 fmt --all --check` 退出 0；`cargo +1.99.0 clippy --workspace --all-targets --offline -- -D warnings`
  无告警（53 单元）；`cargo +1.99.0 test --workspace --release --offline` 全过（`matsuri-core` 18 例）。
  同一组命令此前先用「1.99.0 cargo + 1.98.1 rustc / rustfmt / clippy」代跑过一次，结论一致。
  镜像缺 1.99.0 时的补齐法：`rust-std` / `cargo` / `rustfmt` / `clippy` 走 `rustup toolchain install 1.99.0`，
  `rustc`（84 MB）单组件用分段 `curl -r` 拉 `dist/2026-10-01/rustc-1.99.0-x86_64-unknown-linux-gnu.tar.xz`
  后跑包内 `install.sh --prefix=<toolchain 目录> --disable-verify --disable-ldconfig`。

## 5. 对话核对记录（前期主验证方式）

| 日期 | 核对范围 | 参与人 | 结论 | 遗留 |
|---|---|---|---|---|
| 2026-10-26 | 建仓 / 代号 / master.mdb 表结构 | 用户 + Agent | 代号 `matsuri`、五 crate 建仓、11 张表结构登记 | 机制数值与回合坐标待收口 |
| 2026-10-26 | `single_mode_15_*` / `training_effect` / `text_data` | 用户 + Agent | 数值与效果编号已解码；回合统一 **0 基**（原始 1 基 -1） | gauge 归属 / 合宿延长 / 友人分身 / 双人头 / 事件回合 |
| 2026-10-26 | 合宿延长 / 合宿期支援卡事件 | 用户 + Agent | 每年保证 4 个非比赛合宿回合；正常合宿不触发支援卡事件 | 延长段连续事件 / Lv+1 结算点待测 |
| 2026-10-27 | S2 数据层（11 张表导入 / 校验 / 单测）+ 上游 memo 整理（`f425518`） | 用户 + Agent | 真机 `NeedActGauge = 5`、`MaxActLevel = 10`；turn 一律 0 基；四列全取纪律；撤回 `check_point_effect 4 = スキルPt ÷ 5`（数值巧合，按同号同义读得意率） | `sub_id` 轴 / `gimmick_discount_rate` 单位 / 终局评分是否纳入 AI 目标 |

## 6. CI（`.github/workflows/ci.yml`）

本地三条门依赖一个隐含前提：**引擎仓与本仓并列**（`../umaai_next`）。CI 上原先不成立，2026-10-27 记录如下。

| 现象 | 根因 | 处置 |
|---|---|---|
| `Checkout engine repo` 报 `Input required and not supplied: token` | 仓库名取的是 `github.repository_owner`（本账号）下的 `umaai-next`，该仓不存在，且 `ENGINE_REPO_TOKEN` 未配；`continue-on-error: true` 把失败吞掉 | 改指公开上游 `umaai-project/umaai-next`，token 用 `secrets.ENGINE_REPO_TOKEN \|\| github.token`，**去掉 `continue-on-error`**（缺引擎就该红） |
| `cargo fmt` 挂在 `cargo metadata`：`failed to read .../umaai-matsuri/umaai_next/...`（少一层） | path 依赖指向工作区的**并列目录**，而 `actions/checkout` 只能检进工作区内（`path: umaai_next` 落在仓内） | 检出后 `ln -s "$GITHUB_WORKSPACE/umaai_next" "$GITHUB_WORKSPACE/../umaai_next"` |
| 就算引擎到位也会解析失败 | `rust-toolchain@stable`（= cargo 1.98.1）拒绝装载引擎清单：`umasim = { workspace = true, default-features = false }` 不允许覆盖 workspace 值 | 钉 `dtolnay/rust-toolchain@1.99.0`（与引擎 `rust-toolchain.toml` 一致） |
| clippy / test 覆盖面与本地门不一致 | CI 只写 `--all-targets` / 无 `--workspace` | 补 `--workspace`，与 §4 三条命令逐字对齐（仅省 `--offline`，CI 需要联网取索引） |

- 该「并列 + 软链接」布局在本地用一份干净拷贝实测过：`cargo +1.99.0 metadata` 能解析到引擎清单、
  `cargo +1.99.0 fmt --all --check` 退出 0。
- 引擎侧不改动：CI 只把 `umaai-next` 当 path 依赖编译，不提交、不补丁。
- **2026-10-27 修好并实跑**：run #3（commit `1928fb8`）8 步全成功，`fmt` / `clippy` / `test` 通过，
  `matsuri-core` 18 例（与 §4 本地记录同数）。此前 run #1 / #2 因上述检出问题红。
