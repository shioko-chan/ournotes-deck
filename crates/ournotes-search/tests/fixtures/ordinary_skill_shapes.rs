//! Synthetic release-controlled live scores and judgement-counted support scores.
use super::common::{extend_table, replace_table, set_column};
use super::{data_document, maximum_matrix, roster_document, synthetic_master};
use ournotes_search::{
    engine,
    search::Completion,
    types::{Aggregation, Strategy},
};
use ournotes_sim::{cards::Roster, data::DeckData};
use serde_json::json;

#[test]
fn ordinary_skill_shapes_keep_bounds_and_match_exhaustive() {
    for support in [false, true] {
        let mut s = synthetic_master(6, i64::from(support), 5);
        extend_table(
            &mut s,
            "MasterSkillCondition",
            vec![json!({"_id":9001,"_conditionType":1030,
            "_conditionValues":[1],"_isPositive":true,"_conditionTargetIDs":[42]})],
        );
        extend_table(&mut s, "MasterSkillConditionSet", vec![json!({"_id":9001,"_group":9001,"_conditionIds":[9001]})]);
        extend_table(
            &mut s,
            "MasterSkillCumulativeCondition",
            vec![
                json!({"_id":9001,"_skillCumulativeConditionType":1000,"_conditionValues":[2],"_conditionTargetIDs":[12,13],"_maxCumulativeCount":6}),
                json!({"_id":9002,"_skillCumulativeConditionType":3000,"_conditionTargetIDs":[4,5,6,8],"_maxCumulativeCount":2}),
            ],
        );
        if support {
            extend_table(
                &mut s,
                "MasterSkillEffectSetting",
                vec![json!({"_id":9001,"_skillEffectType":2001,"_phase":2})],
            );
            extend_table(
                &mut s,
                "MasterSkillCondition",
                vec![
                    json!({"_id":9002,"_conditionType":5000,"_conditionValues":[],"_isPositive":true,"_conditionTargetIDs":[30]}),
                    json!({"_id":9003,"_conditionType":5000,"_conditionValues":[],"_isPositive":false,"_conditionTargetIDs":[30]}),
                ],
            );
            extend_table(
                &mut s,
                "MasterSkillConditionSet",
                (9002..=9003).map(|id| json!({"_id":id,"_group":id,"_conditionIds":[id]})).collect(),
            );
            extend_table(
                &mut s,
                "MasterSkillCumulativeCondition",
                vec![json!({"_id":9003,"_skillCumulativeConditionType":1000,
                "_conditionValues":[1],"_conditionTargetIDs":[12,13],"_maxCumulativeCount":12})],
            );
            extend_table(&mut s, "MasterSupportSkillEffect", (1..=5).map(|level|json!({"_id":9000+level,"_supportSkillID":3,"_level":level,
                "_skillTriggerType":1,"_skillTriggerConditionGroup":53,"_skillConditionGroup":9002,"_skillReleaseConditionGroup":0,
                "_skillTargetIDs":[],"_skillEffectType":2001,"_activationTimeSecond":0.5,"_effectValue":1300,"_maxEffectValue":7800,
                "_effectLimitCount":0,"_skillCumulativeConditionID":9003,"_effectExecuteLimitCount":0,"_effectExecuteLimitResetConditionGroup":0})).collect());
            set_column(&mut s, "MasterSupportCard", &mut |r| {
                r["_supportSkillId01"] = json!(3);
                r["_supportSkillId02"] = json!(0);
            });
            set_column(&mut s, "MasterSupportSkillEffect", &mut |r| {
                if r["_supportSkillID"] == 3 && r["_id"].as_i64().unwrap() < 9000 {
                    r["_skillConditionGroup"] = json!(9003);
                    r["_skillEffectType"] = json!(2001);
                    r["_skillCumulativeConditionID"] = json!(9001);
                    r["_effectValue"] = json!(1300);
                    r["_maxEffectValue"] = json!(3900);
                    r["_activationTimeSecond"] = json!(0.5);
                }
            });
        } else {
            set_column(&mut s, "MasterLiveSkillEffect", &mut |r| {
                if r["_liveSkillID"] == 1 {
                    r["_skillEffectType"] = json!(2004);
                    r["_skillTargetIDs"] = json!([12, 13]);
                    r["_skillReleaseConditionGroup"] = json!(9001);
                    r["_activationTimeSecond"] = json!(0.5);
                }
            });
            set_column(&mut s, "MasterMemberCard", &mut |r| {
                if r["_id"] == 1 {
                    r["_leaderSkillID"] = json!(9001);
                }
            });
            extend_table(&mut s,"MasterLeaderSkillEffect", (1..=5).map(|level|json!({"_id":9000+level,"_leaderSkillID":9001,"_level":level,
                "_skillConditionGroup":0,"_skillTargetIDs":[],"_skillEffectType":1501,"_effectValue":2400,"_skillCumulativeConditionID":9002})).collect());
        }
        // No extensions: isolate the two new shapes from the existing synthetic supports.
        if !support {
            replace_table(&mut s, "MasterSupportSkillEffect", json!([]));
        }
        let data = DeckData::from_json(&data_document(&s, 6, i64::from(support), 5).to_string()).unwrap();
        let roster = Roster::from_json(&roster_document(6, i64::from(support), 5).to_string()).unwrap();
        for (aggregation, scene) in [(Aggregation::Maximum, "free"), (Aggregation::Expected, "challenge")] {
            let mut request = maximum_matrix::request(&data, scene, json!({"kind":"score"}));
            request.aggregation = aggregation;
            request.constraints.leader = None;
            request.k = 3;
            request.strategy = Strategy::Exhaustive;
            let oracle = engine::recommend(&data, &roster, &request).unwrap();
            request.strategy = Strategy::BranchAndBound;
            let bounded = engine::recommend(&data, &roster, &request).unwrap();
            assert_eq!(bounded.completion, Completion::Complete);
            assert!(bounded.telemetry.environment.bounds.compiled, "{:?}", bounded.telemetry.environment.bounds);
            assert_eq!(bounded.results, oracle.results, "support={support} aggregation={aggregation:?}");
        }
    }
}
