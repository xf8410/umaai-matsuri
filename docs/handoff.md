# 交接提示词

> 对话上下文很长时，经用户同意后用一段话交接；新上下文只读本文件即可续接。
> 写法：只写「下一个窗口无法从代码 / git 状态里恢复」的信息，不写过程叙述。

- **当前目标**：`matsuri`（骏大祭 / scenarioId 15）——S2 已落地、验证全绿并提交推送，CI 已修绿；
  下一步等用户放行 S3（阶段机，只落已确认坐标）。
- **已完成**：
  - S1 建仓（五 crate / 依赖路径 / 引擎静态数据 / CI / 文档骨架）；
  - S0 数值与口径解码（`single_mode_15_*` 11 张表 + `single_mode_training_effect` + `text_data` 文案）；
    上游 `f425518 更新机制文档(WIP) 1027` 已把 memo 2026-10-27 整理进 `intake.md` §3.9–§3.13 / §7
    （gauge 上限 5 与入账规则、発表会永久 buff、事件时间轴、评分三点锚、稽古基础值口径冲突）；
    本阶段状态仍记 ◐（剩余机制待真机，见 dev_plan §2）；
  - **S2 数据层**：`data/scenario_matsuri.json` 已是真数据，`crates/matsuri-core/src/data.rs` 有
    schema + `validate()` + 11 个内联单测；**结算代码未写**。
- **进行中 / 待办**：
  1. S2 收尾验证**已全绿**（工具链 1.99.0，2026-10-27）：`cargo +1.99.0 fmt --all --check` 退出 0、
     `cargo +1.99.0 clippy --workspace --all-targets --offline -- -D warnings` 无告警、
     `cargo +1.99.0 test --workspace --release --offline` 18 例通过（数据层 11 + 配置 4 + 阶段 2 + 训练员 1）。
     镜像原本只有 1.98.1（且 1.98.1 的 cargo 装载不动引擎清单），已把 1.99.0 的 rustc / rustfmt / clippy 组件补齐；
     补齐步骤见 `tests_overview.md` §4，**下一个沙箱若没补，三条命令一律加 `+1.99.0` 前先装 toolchain**。
  2. S2 已提交并推送：`main` = `95a8d36`（父 = 上游 `f425518`）；后续仍遵守 AGENTS：不自动 `commit` / `push`；
  3. 仓库形态**已按用户要求处理**：原 `xf8410/umaai-matsuri` 是 fork（GitHub 无「脱 fork」API）→
     已把该 fork **改名**为 `xf8410/umaai-matsuri-fork-backup`（未删除，用户可自行决定删除），
     再新建**同名普通仓**（API 确认 `fork = false`、无 `parent`）并推上游全量历史；`origin` 现指向该普通仓；
  4. CI **已修并转绿**（run #3 / commit `1928fb8`，8 步全成功、18 例通过；原先必红，原因见 `tests_overview.md` §6）：
     引擎检出改指公开上游 `umaai-project/umaai-next` 并去掉 `continue-on-error`；检出后补
     `$GITHUB_WORKSPACE/../umaai_next` 软链接满足 `../umaai_next` path 依赖；工具链由 `@stable` 改钉 `@1.99.0`；
     clippy / test 加 `--workspace` 与本地门一致。
- **数据再生方式**：资产由上传的「駿大祭机制包」转换而来，转换脚本在**会话工作区**（不在仓内）；
  沙箱销毁后需重建脚本时，以 `plan/intake.md` §3 / §4 与 `data.rs` 的 schema 为准，
  训练基础值两处来源在包内 `参考实现` 与数据表之间冲突 → **一律以 `intake.md` §3.2（数据表）为准**。
- **关键文件**：`docs/plan/intake.md`（§4 落库口径、§3.10 评分仅外部评价、§7 已确认 / 存疑清单）、
  `docs/plan/stubs.md`（含「观测值不入库」表）、`docs/plan/scenario_dev_plan.md`、
  `crates/matsuri-core/src/data.rs`、`data/scenario_matsuri.json`。
- **常用命令**：`cargo +1.99.0 fmt --all --check`、`cargo +1.99.0 clippy --workspace --all-targets --offline -- -D warnings`、
  `cargo +1.99.0 test --workspace --release --offline`、`cargo +1.99.0 run --release --bin matsuri -- manual`。
- **约束 / 未决**：不改引擎仓一行（提交前确认引擎 `git status` 为空）；机制未确认不得用 stub 绕过；
  不照搬旧剧本（拉面 / scenario 14）机制与 `effect_type` 语义；观测值（2.59×、评分系数、
  発表会走势）不入数据与配置；不自动 `commit` / `push`；提交前统一更新 README / 文档；
  待真机：gauge 彩圈范围、合宿 Lv+1 结算点、延长段支援卡事件、友人分身、同名双人头、
  稽古基础值口径（`sub_id` vs `command_level`，intake §5 #21）、成功档一次性奖励、训练公式下段、
  目标赛事 SP（+72？）、`gimmick_discount_rate` 单位（intake §5 #26）；
  事件触发回合已按 memo 基本收口（intake §3.13，Senior 10 月前半待核）。
