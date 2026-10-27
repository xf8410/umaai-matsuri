//! 剧本资产（`scenario_matsuri.json`）：schema、载入与校验
//!
//! 剧本数据**不进引擎仓**：根目录由宿主注入（终端 / Web / Android 各按自家布局），
//! 本模块只负责「从给定位置读 + 校验」，不碰任何写死路径。
//!
//! 内容是 `master.mdb` 的 `single_mode_15_*` 十一张表 + `single_mode_training_effect`（15）
//! 导入（数值不改，只统一形状：`effect_type` + `condition_*` + 四值收成一个 [`Effect`]）。
//! **口径**：turn 一律 **0 基**（原表 1 基，导入即 −1）；`effect_value_1..4` 收成
//! `values[4]`，读取必须四列全取（值可能只落在其中一列）。
//! **这里只有数据，没有结算**：`effect_type` 在 scenario 15 内**全局同义**（intake §5 #18），
//! 跨剧本不套用；`result_state` 按**所在表**定档。详见
//! [`docs/mechanism.md`](../../../docs/mechanism.md) §3 与 [`docs/plan/intake.md`](../../../docs/plan/intake.md) §4；
//! 未收口项见 [`docs/plan/stubs.md`](../../../docs/plan/stubs.md)。

use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use serde::Deserialize;

use crate::MAX_TURN;

/// 剧本数据文件名（放在宿主注入的剧本数据根下）
pub const MATSURI_DATA_FILE: &str = "scenario_matsuri.json";

/// 演目等级上限（`act_effect` 覆盖 Lv1–10；友人稀有度决定实际可用上限）
pub const ACT_LEVEL_MAX: i32 = 10;

/// 骏大祭全部静态数据
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatsuriData {
    /// 剧本 ID（骏大祭为 15）
    pub scenario_id: i32,
    /// 剧本开始时刻（`single_mode_scenario`，JST 2026-10-26 12:00）；日历换算用，不参与结算
    pub start_date_unix: i64,
    /// 普通支援卡位（`single_mode_scenario`）；固定友人另计，不在此数内
    pub support_num: i32,
    /// 演目ゲージ上限（真机 `NeedActGauge = 5`，见 intake §3.8）
    pub act_gauge_max: i32,
    /// 演目 Lv 上限（SSR 吾妻档 `MaxActLevel = 10`；R / 不带卡的 6 待真机，不入资产）
    pub act_level_max: i32,
    /// 剧本数值上限（原表列名，语义口径见 `docs/plan/stubs.md`）
    pub stage: StageCaps,
    /// 五名编成外担当（`single_mode_15_act_member`）：演目 ↔ 马娘 ↔ 训练 ID
    pub acts: Vec<ActMember>,
    /// 演目等级效果（`single_mode_15_act_effect`）：**无 `act_type` 列**，五演目共用一条曲线
    pub act_effect: Vec<ActEffectRow>,
    /// 進捗発表会 / 本番排程（`single_mode_15_schedule`；turn 已转 0 基）
    pub schedule: Vec<ScheduleRow>,
    /// 伝受窗口回合（`single_mode_15_advice_recommend`；turn 已转 0 基 = 2 / 11 / 23 / 35 / 47 / 59）
    pub advice_events: Vec<i32>,
    /// 教え一览（`single_mode_15_advice`）
    pub advice: Vec<AdviceRow>,
    /// 教え效果（`single_mode_15_advice_effect`）
    pub advice_effect: Vec<AdviceEffectRow>,
    /// 転成效果（`single_mode_15_fusion_effect`）
    pub fusion_effect: Vec<FusionEffectRow>,
    /// 発表会 / 本番结算效果（`single_mode_15_check_point_effect`）
    pub check_point_effect: Vec<CheckPointEffectRow>,
    /// 本番演出效果（`single_mode_15_finals_act_effect`）
    pub finals_act_effect: Vec<FinalsEffectRow>,
    /// 出行加级（`single_mode_15_outing_effect`）
    pub outing_effect: Vec<OutingEffectRow>,
    /// 普通训练基础值表 `[训练 0-4][档位] -> [速 耐 力 根 智 PT 体力消耗]`
    /// （`single_mode_training_effect`，`sub_id = 1`；其余档位待重提取，见 docs/plan/stubs.md）
    pub training_basic_value: Vec<Vec<[i32; 7]>>,
    /// 演目稽古基础值（命令 `5101`–`5105`，来自 `single_mode_training_effect`）
    pub act_training_basic: Vec<ActTrainingRow>,
    /// STUB: 占位资源，演目 gauge 账本定形后替换（见 docs/plan/stubs.md）
    pub sample_resource: i32,
}

/// 剧本数值上限（`single_mode_scenario` 行 15 原值）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageCaps {
    /// 原表 `max_speed`（不是官方「基础上限」口径，两者不同源）
    pub max_speed: i32,
    /// 原表 `max_stamina`
    pub max_stamina: i32,
    /// 原表 `max_pow`
    pub max_pow: i32,
    /// 原表 `max_guts`
    pub max_guts: i32,
    /// 原表 `max_wiz`
    pub max_wiz: i32,
    /// 原表 `upper_limit_*`：`[速 耐 力 根 智]` 绝对上限
    pub upper_limit: [i32; 5],
    /// 原表 `max_skill_point`（技能点上限）
    pub max_skill_point: i32,
    /// 原表 `gimmick_discount_rate`（剧本折扣，单位待收口：机制包按「−10%」用）
    pub gimmick_discount_rate: i32,
}

/// 一条效果行的原始列：条件 + 效果类型 + 四值
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Effect {
    /// 原表 `condition_type`：仅 `1` 配 `condition_value = 5`（编成支援卡类型 ≥5）已由官方帮助对上；
    /// `1` / `2` / `3` 的其余取值语义**未解码**，此处不作断言（见 intake §3.3 / §7 存疑清单）
    #[serde(default)]
    pub condition_type: i32,
    /// 原表 `condition_value`（随 `condition_type` 变义；`1 / 5` = 编成支援卡类型数）
    #[serde(default)]
    pub condition_value: i32,
    /// 原表 `effect_type`：scenario 15 内全局同义（`202` 友情 / `4` 得意率 / `2` 灵感 / `204` 比赛…），
    /// 不套用剧本 14 / 12 的语义（intake §3.6 / §5 #18）
    pub effect_type: i32,
    /// 原表 `effect_value_1..4`；同一类型可能只填其中一列，读取时四列都要看
    pub values: [i32; 4],
}

/// 一名编成外担当神駒
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActMember {
    /// 演目编号（1 流鏑馬 / 2 神旗争奪戦 / 3 納舞 / 4 大曳神輿 / 5 飛駆）
    pub act_type: i32,
    /// 担当马娘 ID
    pub chara_id: i32,
    /// 担当马娘名（`text_data` 原文）
    pub chara_name: String,
    /// 原表 `priority`（当前数据恒为 100）
    pub priority: i32,
    /// 对应训练 `command_id`（真机报文里演目稽古仍走该训练 ID）
    pub command_id: i32,
}

/// 演目等级效果行（表内无 `act_type` / `id` 列）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActEffectRow {
    /// 演目等级（1–10）
    pub level: i32,
    /// 效果原始列
    pub effect: Effect,
}

/// 一次稽古节点（進捗発表会 / 本番）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduleRow {
    /// 排程 ID（`check_point_effect.schedule_id` 用它索引）
    pub id: i32,
    /// 结算回合（0 基）
    pub turn: i32,
    /// 需达标的演目条数
    pub need_act_num: i32,
    /// 每条演目需达到的等级
    pub need_act_level: i32,
}

/// 一种教え
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdviceRow {
    /// 教え ID（101–501 为初始五种，601–1001 为追加五种）
    pub id: i32,
    /// 原表 `fusion_type`：1 = 可作転成材料，2 = 只作受体
    pub fusion_type: i32,
    /// 原表 `add_schedule_type`：新增窗口序号（1 开局 / 2 / 3）
    pub add_schedule_type: i32,
}

/// 教え效果行
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdviceEffectRow {
    /// 原表 `id`
    pub id: i32,
    /// 所属教え ID
    pub advice_id: i32,
    /// 教え等级（1–10）
    pub level: i32,
    /// 效果原始列
    pub effect: Effect,
}

/// 転成效果行（材料教え被消耗时附加给受体教え）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FusionEffectRow {
    /// 原表 `id`
    pub id: i32,
    /// 材料教え ID
    pub advice_id: i32,
    /// 效果原始列
    pub effect: Effect,
}

/// 発表会 / 本番结算效果行（原表无 `condition_*` 列）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckPointEffectRow {
    /// 原表 `id`
    pub id: i32,
    /// 所属排程 ID；`0` = 「第 1 回前」基线行（得意率 +20，intake §3.6 / §5 #8 已收口）
    pub schedule_id: i32,
    /// 结果档：0 = 未达 / 1 = 成功 / 2 = 大成功（3 未观察到）
    pub result_state: i32,
    /// 效果原始列
    pub effect: Effect,
}

/// 本番演出效果行
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinalsEffectRow {
    /// 原表 `id`
    pub id: i32,
    /// 效果原始列
    pub effect: Effect,
}

/// 出行（友人「吾妻」）加级行
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutingEffectRow {
    /// 原表 `id`
    pub id: i32,
    /// 支援卡 ID（SSR 30323 / R 10149）
    pub support_card_id: i32,
    /// 友人剧情段（1–5）
    pub story_step: i32,
    /// 该段给最低演目加的等级
    pub add_level: i32,
}

/// 演目稽古基础值行（`single_mode_training_effect`，`result_state = 2`）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActTrainingRow {
    /// 演目命令 ID（`5101`–`5105`，与 [`MatsuriData::acts`] 的 `command_id` 同源）
    pub command_id: i32,
    /// 提取档位（原表 `sub_id`，当前仅 `sub_id = 1` 一档）；暂按静态表 `command_level` = 演目 Lv 使用，
    /// 两口径冲突待真机收口（intake §5 #21 / glossary 2026-10-27 行）
    pub level: i32,
    /// `[速 耐 力 根 智 PT 体力消耗]`
    pub values: [i32; 7],
}

impl MatsuriData {
    /// 从宿主注入的数据根载入 `scenario_matsuri.json`
    pub fn load(root: impl AsRef<Path>) -> Result<Self> {
        let path = root.as_ref().join(MATSURI_DATA_FILE);
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("读取剧本数据失败: {}", path.display()))?;
        Self::parse(&text).with_context(|| format!("剧本数据来源: {}", path.display()))
    }

    /// 解析并校验剧本数据（测试 / 内嵌场景复用）
    pub fn parse(text: &str) -> Result<Self> {
        let data: Self = serde_json::from_str(text).context("解析剧本数据失败")?;
        data.validate()?;
        Ok(data)
    }

    /// 载入期校验：只查表形状与跨表引用（ID 对得上、等级不缺、回合有序），不做机制结算
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.scenario_id == 15,
            "scenario_id 应为 15，实际 {}",
            self.scenario_id
        );
        ensure!(
            self.support_num >= 1,
            "support_num 至少 1，实际 {}",
            self.support_num
        );
        for (i, cap) in self.stage.upper_limit.iter().enumerate() {
            ensure!(*cap >= 0, "stage.upper_limit[{i}] 不能为负，实际 {cap}");
        }
        self.validate_acts()?;
        self.validate_act_effect()?;
        self.validate_schedule()?;
        self.validate_advice()?;
        self.validate_check_point()?;
        self.validate_outing()?;
        self.validate_training_basic_value()?;
        self.validate_act_training_basic()
    }

    /// 担当表：五名、演目号 1–5 唯一、马娘与训练 ID 不重；gauge / Lv 上限非零
    fn validate_acts(&self) -> Result<()> {
        ensure!(
            self.acts.len() == 5,
            "acts 应有 5 名担当，实际 {}",
            self.acts.len()
        );
        ensure!(
            self.act_gauge_max >= 1,
            "act_gauge_max 应 ≥1，实际 {}",
            self.act_gauge_max
        );
        ensure!(
            (1..=ACT_LEVEL_MAX).contains(&self.act_level_max),
            "act_level_max 应在 1–{ACT_LEVEL_MAX}，实际 {}",
            self.act_level_max
        );
        let (mut act_types, mut chara_ids, mut command_ids) =
            (HashSet::new(), HashSet::new(), HashSet::new());
        for act in &self.acts {
            ensure!(
                (1..=5).contains(&act.act_type),
                "act_type 应在 1–5，实际 {}",
                act.act_type
            );
            ensure!(
                !act.chara_name.is_empty(),
                "act_type {} 缺担当名称",
                act.act_type
            );
            ensure!(
                act_types.insert(act.act_type),
                "act_type {} 重复",
                act.act_type
            );
            ensure!(
                chara_ids.insert(act.chara_id),
                "chara_id {} 重复",
                act.chara_id
            );
            ensure!(
                command_ids.insert(act.command_id),
                "command_id {} 重复",
                act.command_id
            );
        }
        Ok(())
    }

    /// 演目效果：等级 1–10 全覆盖，同等级内效果类型不重
    fn validate_act_effect(&self) -> Result<()> {
        ensure!(!self.act_effect.is_empty(), "act_effect 不能为空");
        for row in &self.act_effect {
            ensure!(
                (1..=ACT_LEVEL_MAX).contains(&row.level),
                "act_effect 等级越界: {}（应为 1–{ACT_LEVEL_MAX}）",
                row.level
            );
        }
        for level in 1..=ACT_LEVEL_MAX {
            let rows: Vec<&ActEffectRow> = self
                .act_effect
                .iter()
                .filter(|r| r.level == level)
                .collect();
            ensure!(!rows.is_empty(), "act_effect 缺 Lv{level}");
            let mut seen = HashSet::new();
            for row in &rows {
                ensure!(
                    seen.insert(row.effect.effect_type),
                    "act_effect Lv{level} 效果类型重复: {}",
                    row.effect.effect_type
                );
            }
        }
        Ok(())
    }

    /// 稽古节点：ID 与回合同号递增，回合落在合法范围，门槛不超演目数与等级上限
    fn validate_schedule(&self) -> Result<()> {
        ensure!(!self.schedule.is_empty(), "schedule 不能为空");
        for (i, row) in self.schedule.iter().enumerate() {
            ensure!(
                row.id == i as i32 + 1,
                "schedule[{i}] 的 id 应为 {}，实际 {}",
                i + 1,
                row.id
            );
            ensure!(
                (0..=MAX_TURN).contains(&row.turn),
                "schedule[{}] 回合越界: {}（0–{MAX_TURN}）",
                row.id,
                row.turn
            );
            ensure!(
                (1..=self.acts.len() as i32).contains(&row.need_act_num),
                "schedule[{}] need_act_num 应在 1–{}，实际 {}",
                row.id,
                self.acts.len(),
                row.need_act_num
            );
            ensure!(
                (1..=ACT_LEVEL_MAX).contains(&row.need_act_level),
                "schedule[{}] need_act_level 应在 1–{ACT_LEVEL_MAX}，实际 {}",
                row.id,
                row.need_act_level
            );
            if i > 0 {
                ensure!(
                    self.schedule[i - 1].turn < row.turn,
                    "schedule 回合必须升序: [{}] {} !< {}",
                    row.id,
                    self.schedule[i - 1].turn,
                    row.turn
                );
            }
        }
        Ok(())
    }

    /// 教诲三表：ID 唯一、效果行的 `advice_id` 必须能在教诲表里找到、等级不越界
    fn validate_advice(&self) -> Result<()> {
        ensure!(!self.advice.is_empty(), "advice 不能为空");
        let ids: HashSet<i32> = self.advice.iter().map(|a| a.id).collect();
        ensure!(ids.len() == self.advice.len(), "advice 存在重复 id");
        for a in &self.advice {
            ensure!(
                a.fusion_type >= 1,
                "advice[{}] fusion_type 应 ≥1，实际 {}",
                a.id,
                a.fusion_type
            );
            ensure!(
                a.add_schedule_type >= 1,
                "advice[{}] add_schedule_type 应 ≥1，实际 {}",
                a.id,
                a.add_schedule_type
            );
        }
        for pair in self.advice_events.windows(2) {
            ensure!(
                pair[0] < pair[1],
                "advice_events 回合必须升序: {} !< {}",
                pair[0],
                pair[1]
            );
        }
        for turn in &self.advice_events {
            ensure!(
                (0..=MAX_TURN).contains(turn),
                "advice_events 回合越界: {turn}（0–{MAX_TURN}）"
            );
        }
        ensure!(!self.advice_effect.is_empty(), "advice_effect 不能为空");
        let mut seen_ids = HashSet::new();
        let mut seen_keys = HashSet::new();
        for row in &self.advice_effect {
            ensure!(seen_ids.insert(row.id), "advice_effect id 重复: {}", row.id);
            ensure!(
                ids.contains(&row.advice_id),
                "advice_effect[{}] 指向未知教诲 {}",
                row.id,
                row.advice_id
            );
            ensure!(
                (1..=ACT_LEVEL_MAX).contains(&row.level),
                "advice_effect[{}] 等级越界: {}",
                row.id,
                row.level
            );
            ensure!(
                seen_keys.insert((row.advice_id, row.level, row.effect.effect_type)),
                "advice_effect 教诲 {} Lv{} 效果类型重复: {}",
                row.advice_id,
                row.level,
                row.effect.effect_type
            );
        }
        let mut fusion_ids = HashSet::new();
        for row in &self.fusion_effect {
            ensure!(
                fusion_ids.insert(row.id),
                "fusion_effect id 重复: {}",
                row.id
            );
            ensure!(
                ids.contains(&row.advice_id),
                "fusion_effect[{}] 指向未知教诲 {}",
                row.id,
                row.advice_id
            );
        }
        let mut finals_ids = HashSet::new();
        for row in &self.finals_act_effect {
            ensure!(
                finals_ids.insert(row.id),
                "finals_act_effect id 重复: {}",
                row.id
            );
        }
        Ok(())
    }

    /// 発表会效果：`schedule_id` 要么 0（未归类），要么在排程表里；结果档只允许 0–2
    fn validate_check_point(&self) -> Result<()> {
        ensure!(
            !self.check_point_effect.is_empty(),
            "check_point_effect 不能为空"
        );
        let schedule_ids: HashSet<i32> = self.schedule.iter().map(|s| s.id).collect();
        let mut seen = HashSet::new();
        for row in &self.check_point_effect {
            ensure!(
                seen.insert(row.id),
                "check_point_effect id 重复: {}",
                row.id
            );
            ensure!(
                row.schedule_id == 0 || schedule_ids.contains(&row.schedule_id),
                "check_point_effect[{}] 指向未知排程 {}",
                row.id,
                row.schedule_id
            );
            ensure!(
                (0..=2).contains(&row.result_state),
                "check_point_effect[{}] result_state 越界: {}（3 档未观察到，出现即复核）",
                row.id,
                row.result_state
            );
        }
        Ok(())
    }

    /// 出行表：卡与剧情段非空，剧情段 1–5
    fn validate_outing(&self) -> Result<()> {
        ensure!(!self.outing_effect.is_empty(), "outing_effect 不能为空");
        let mut seen = HashSet::new();
        for row in &self.outing_effect {
            ensure!(seen.insert(row.id), "outing_effect id 重复: {}", row.id);
            ensure!(
                row.support_card_id > 0,
                "outing_effect[{}] 卡 ID 非法: {}",
                row.id,
                row.support_card_id
            );
            ensure!(
                (1..=5).contains(&row.story_step),
                "outing_effect[{}] 剧情段越界: {}",
                row.id,
                row.story_step
            );
            ensure!(
                row.add_level >= 1,
                "outing_effect[{}] add_level 应 ≥1，实际 {}",
                row.id,
                row.add_level
            );
        }
        Ok(())
    }

    /// 普通训练基础值表（`sub_id = 1` 档已提取，Lv2–5 待补，见 docs/plan/stubs.md）：
    /// 五个训练、每训练至少一档、每档七值
    fn validate_training_basic_value(&self) -> Result<()> {
        ensure!(
            self.training_basic_value.len() == 5,
            "training_basic_value 应有 5 个训练，实际 {}",
            self.training_basic_value.len()
        );
        for (i, levels) in self.training_basic_value.iter().enumerate() {
            ensure!(!levels.is_empty(), "training_basic_value[{i}] 至少应有一档");
        }
        Ok(())
    }

    /// 演目稽古基础值：五命令、与担当表的 `command_id` 一一对应、`(命令, 档位)` 不重复
    fn validate_act_training_basic(&self) -> Result<()> {
        ensure!(
            self.act_training_basic.len() == 5,
            "act_training_basic 应有 5 个演目命令，实际 {}",
            self.act_training_basic.len()
        );
        let commands: HashSet<i32> = self.acts.iter().map(|a| a.command_id).collect();
        let mut seen = HashSet::new();
        for row in &self.act_training_basic {
            ensure!(
                commands.contains(&row.command_id),
                "act_training_basic 命令 {} 不在担当表内",
                row.command_id
            );
            ensure!(
                row.level >= 1,
                "act_training_basic[{}] 档位应 ≥1，实际 {}",
                row.command_id,
                row.level
            );
            ensure!(
                seen.insert((row.command_id, row.level)),
                "act_training_basic 命令 {} 档位 {} 重复",
                row.command_id,
                row.level
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 仓内数据原文（测试改动形状用）
    fn repo_text() -> String {
        std::fs::read_to_string(crate::testing::data_dir().join(MATSURI_DATA_FILE))
            .expect("仓内应有 scenario_matsuri.json")
    }

    /// 仓库自带数据可载入，且各表行数与接入包登记一致、回合坐标已转 0 基
    #[test]
    fn load_repo_data() -> Result<()> {
        let data = MatsuriData::load(crate::testing::data_dir())?;
        assert_eq!(data.scenario_id, 15);
        assert_eq!(data.act_gauge_max, 5);
        assert_eq!(data.act_level_max, ACT_LEVEL_MAX);
        assert_eq!(data.acts.len(), 5);
        assert_eq!(data.act_effect.len(), 66);
        assert_eq!(data.schedule.len(), 3);
        assert_eq!(data.advice_events.len(), 6);
        assert_eq!(data.advice.len(), 10);
        assert_eq!(data.advice_effect.len(), 153);
        assert_eq!(data.fusion_effect.len(), 9);
        assert_eq!(data.check_point_effect.len(), 20);
        assert_eq!(data.finals_act_effect.len(), 4);
        assert_eq!(data.outing_effect.len(), 10);
        assert_eq!(data.act_training_basic.len(), 5);
        // 0 基口径（intake §3.1：原始 1 基 24 / 48 / 68 与 3 / 12 / 24 / 36 / 48 / 60）
        assert_eq!(
            data.schedule.iter().map(|s| s.turn).collect::<Vec<_>>(),
            vec![23, 47, 67]
        );
        assert_eq!(data.advice_events, vec![2, 11, 23, 35, 47, 59]);
        println!("十一张表导入通过：担当 5 / 演目效果 66 / 教诲效果 153 / 転成 9 / 発表会 20");
        Ok(())
    }

    /// 读取纪律：同一 `effect_type` 的值未必落在同一列 → 结算必须四列全取
    #[test]
    fn effect_values_land_in_different_columns() -> Result<()> {
        let data = MatsuriData::load(crate::testing::data_dir())?;
        let value_cols = |advice_id: i32, effect_type: i32| -> Vec<usize> {
            let mut cols = data
                .advice_effect
                .iter()
                .filter(|r| r.advice_id == advice_id && r.effect.effect_type == effect_type)
                .flat_map(|r| {
                    r.effect
                        .values
                        .iter()
                        .enumerate()
                        .filter(|(_, v)| **v != 0)
                        .map(|(i, _)| i)
                })
                .collect::<Vec<_>>();
            cols.sort_unstable();
            cols.dedup();
            cols
        };
        // `13`（稽古主基础 %）：101 只填第 4 列，601 同时填第 2 与第 4 列
        assert_eq!(value_cols(101, 13), vec![3]);
        assert_eq!(value_cols(601, 13), vec![1, 3]);
        Ok(())
    }

    /// 未知字段严格报错
    #[test]
    fn reject_unknown_field() {
        let bad = repo_text().replace("\"scenario_id\"", "\"scenario_idx\"");
        assert!(MatsuriData::parse(&bad).is_err(), "未知字段应报错");
    }

    /// 演目等级缺失要报错（结算按等级取行，缺档不允许）
    #[test]
    fn reject_missing_act_level() {
        let text = repo_text();
        let cut = text
            .lines()
            .filter(|l| !l.contains(r#"{"level": 3, "effect""#))
            .collect::<Vec<_>>()
            .join("\n");
        let mut data: MatsuriData = serde_json::from_str(&cut).expect("去掉 Lv3 后仍应可解析");
        data.act_effect.retain(|r| r.level != 3);
        assert!(data.validate().is_err(), "缺 Lv3 应报错");
    }

    /// `result_state = 3` 未观察到，出现即报错复核
    #[test]
    fn reject_unobserved_result_state() {
        let mut data: MatsuriData = serde_json::from_str(&repo_text()).unwrap();
        data.check_point_effect[0].result_state = 3;
        assert!(data.validate().is_err(), "未观察到的结果档应报错");
    }

    /// 排程回合必须升序
    #[test]
    fn reject_unsorted_schedule() {
        let mut data: MatsuriData = serde_json::from_str(&repo_text()).unwrap();
        data.schedule.reverse();
        assert!(data.validate().is_err(), "排程逆序应报错");
    }

    /// 效果行的教诲引用必须能对上
    #[test]
    fn reject_dangling_advice_id() {
        let mut data: MatsuriData = serde_json::from_str(&repo_text()).unwrap();
        data.advice_effect[0].advice_id = 9999;
        assert!(data.validate().is_err(), "未知教诲 ID 应报错");
    }

    /// 训练表形状：六个训练应报错
    #[test]
    fn reject_wrong_training_count() {
        let mut data: MatsuriData = serde_json::from_str(&repo_text()).unwrap();
        data.training_basic_value.push(vec![[0; 7]]);
        assert!(data.validate().is_err(), "六个训练应报错");
    }

    /// 训练表形状：六值行应报错（serde 阶段就拒）
    #[test]
    fn reject_bad_row_width() {
        let bad = repo_text().replace("[12, 0, 2, 0, 0, 7, -20]", "[12, 0, 2, 0, 0, 7]");
        assert!(MatsuriData::parse(&bad).is_err(), "六值行应报错");
    }

    /// 演目稽古命令必须与担当表对得上（防串表）
    #[test]
    fn reject_unknown_act_command() {
        let mut data: MatsuriData = serde_json::from_str(&repo_text()).unwrap();
        data.act_training_basic[0].command_id = 5199;
        assert!(data.validate().is_err(), "担当表外的命令应报错");
    }

    /// gauge 上限为 0 会让演目稽古永不点亮，属越界数据
    #[test]
    fn reject_zero_gauge_max() {
        let mut data: MatsuriData = serde_json::from_str(&repo_text()).unwrap();
        data.act_gauge_max = 0;
        assert!(data.validate().is_err(), "act_gauge_max = 0 应报错");
    }
}
