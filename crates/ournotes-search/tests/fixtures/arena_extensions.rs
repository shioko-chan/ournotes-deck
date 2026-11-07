//! Public request regressions using synthetic data only.
use super::*;
use ournotes_search::{
    engine,
    handler::build_card_pool,
    types::{PlayPolicy, RecommendationRequest},
};

fn inputs(snaps: i64) -> (DeckData, Roster, RecommendationRequest) {
    let mut synth = synthetic_master(5, snaps, 5);
    set_column(&mut synth, "MasterLiveMusic", &mut |r| {
        for key in ["_gekisouMission1", "_gekisouMission2", "_gekisouMission3"] {
            r[key] = json!(3);
        }
    });
    let data = DeckData::from_json(&data_document(&synth, 5, snaps, 5).to_string()).unwrap();
    let roster = Roster::from_json(&roster_document(5, snaps, 5).to_string()).unwrap();
    let mut request = joint_request("free", true, json!({"kind":"score"}));
    request.k = 1;
    (data, roster, request)
}

#[test]
fn arena_include_snaps_validates_and_filters_every_result() {
    let (data, roster, mut request) = inputs(1);
    request.execution = ournotes_search::types::Execution::Power { music_id: None, event_parameter: false };
    request.scenario = None;
    request.context = None;
    request.metric = Metric::Power;
    request.constraints.include_snaps = vec![1, 1];
    request.k = 5;
    let bounded = engine::recommend(&data, &roster, &request).unwrap();
    assert_eq!(bounded.results.len(), 5);
    assert!(bounded.results.iter().all(|d| d.snaps.contains(&Some(1))));
    request.strategy = Strategy::Exhaustive;
    assert_eq!(bounded.results, engine::recommend(&data, &roster, &request).unwrap().results);
    for (ids, excluded, no_snaps) in [(vec![99], vec![], false), (vec![1], vec![1], false), (vec![1], vec![], true)] {
        request.constraints.include_snaps = ids;
        request.constraints.exclude_snaps = excluded;
        request.constraints.no_snaps = no_snaps;
        assert!(build_card_pool(&data, &roster, &request).is_err());
    }
    let parsed: RecommendationRequest = serde_json::from_value({
        let mut value = joint_request_json("free", true, json!({"kind":"score"}));
        value["constraints"]["includeSnaps"] = json!([1]);
        value
    })
    .unwrap();
    assert_eq!(parsed.constraints.include_snaps, [1]);
}

#[test]
fn arena_rank_override_matches_master_and_exhaustive_with_diagnostics() {
    let (data, roster, mut request) = inputs(0);
    let baseline = engine::recommend(&data, &roster, &request).unwrap();
    assert!(!serde_json::to_value(&baseline.results[0]).unwrap().as_object().unwrap().contains_key("gekisouRanges"));
    let percents = [190, 220, 250];
    request.context.as_mut().unwrap().gekisou_rank_percents = Some(percents);
    let bounded = engine::recommend(&data, &roster, &request).unwrap();
    let mut master_data = data.clone();
    for row in &mut master_data.master.gekisou_ranking_score_bonuses {
        if row.rank == 1 {
            row.score_bonus_percent = percents[row.count as usize - 1];
        }
    }
    let mut master_request = request.clone();
    master_request.context.as_mut().unwrap().gekisou_rank_percents = None;
    assert_eq!(bounded.results, engine::recommend(&master_data, &roster, &master_request).unwrap().results);
    request.strategy = Strategy::Exhaustive;
    assert_eq!(bounded.results, engine::recommend(&data, &roster, &request).unwrap().results);
    request.context.as_mut().unwrap().gekisou_range_diagnostics = true;
    let out = engine::recommend(&data, &roster, &request).unwrap();
    let ranges = out.results[0].gekisou_ranges.as_ref().unwrap();
    assert_eq!(ranges.len(), 3);
    for (i, r) in ranges.iter().enumerate() {
        assert_eq!(r.range_index, i);
        assert_eq!(r.rank_percent, Some(percents[i]));
        assert_eq!(r.mission, 3);
        assert!(r.just_count >= r.raw_just);
    }
    request.context.as_mut().unwrap().gekisou_rank_percents = Some([-1, 0, 0]);
    assert!(engine::recommend(&data, &roster, &request).is_err());
}

#[test]
fn arena_timing_wire_rejects_invalid_inputs() {
    for play in [
        json!({"kind":"timingError","sigmaMs":0,"greatFraction":0}),
        json!({"kind":"timingError","sigmaMs":-1,"greatFraction":0}),
        json!({"kind":"timingError","sigmaMs":10,"greatFraction":2}),
        json!({"kind":"timingError","sigmaMs":10}),
        json!({"kind":"timingError","sigmaMs":10,"greatFraction":0,"justFraction":1}),
    ] {
        assert!(serde_json::from_value::<PlayPolicy>(play).is_err());
    }
    assert!(
        serde_json::from_value::<PlayPolicy>(json!({"kind":"timingError","sigmaMs":10,"greatFraction":0.1})).is_ok()
    );
}

#[test]
fn arena_timing_error_search_values_4004_and_preserves_accuracy() {
    let (mut data, roster, mut request) = inputs(1);
    // Install a real Gekisou-support 4004 row using the master's native row types.
    for m in &mut data.master.member_cards {
        m.gekisou_skill_id = 1;
    }
    data.master.gekisou_skill_effects.clear();
    for s in &mut data.master.support_cards {
        s.support_skill_id_01 = 0;
        s.support_skill_id_02 = 0;
        s.gekisou_support_skill_id_01 = 901;
        s.gekisou_support_skill_id_02 = 0;
    }
    for r in &mut data.master.support_card_ranks {
        r.gekisou_support_skill_01_level = 1;
    }
    data.master
        .gekisou_support_skills
        .push(serde_json::from_value(json!({"_id":901,"_gekisouMissionType":3})).unwrap());
    data.master.gekisou_support_skill_effects.push(
        serde_json::from_value(json!({
            "_id":901,"_gekisouSupportSkillID":901,"_level":1,"_skillTriggerType":1,
            "_skillTriggerConditionGroup":53,"_skillEffectType":4004,"_effectValue":10000,
            "_activationTimeSecond":2.0,"_skillTargetIDs":[]
        }))
        .unwrap(),
    );
    data.master.live_judgement_timings.push(
        serde_json::from_value(json!({
            "_id":999,"_noteJudgementType":1,"_noteSimulateJudgement":1,"_beforeMs":200,"_afterMs":200
        }))
        .unwrap(),
    );
    data.master
        .skill_effect_settings
        .push(serde_json::from_value(json!({"_id":999,"_skillEffectType":4004,"_phase":2})).unwrap());
    for r in &mut data.master.judgement_parameters {
        if r.note_simulate_judgement == 6 {
            r.score_percent = 200;
        }
    }
    data.master.reindex().unwrap();
    request.context.as_mut().unwrap().gekisou_range_diagnostics = true;
    request.execution = ournotes_search::types::Execution::Live {
        score_id: SCORE_ID,
        gekisou: true,
        play: PlayPolicy::TimingError { sigma_ms: 60.0, great_fraction: 0.0 },
    };
    request.constraints.include_snaps = vec![1];
    let active = engine::recommend(&data, &roster, &request).unwrap();
    let mut inactive_data = data.clone();
    inactive_data.master.gekisou_support_skill_effects.last_mut().unwrap().effect_value = 0;
    let inactive = engine::recommend(&inactive_data, &roster, &request).unwrap();
    let raw = |out: &ournotes_search::types::RecommendationOutcome| {
        out.results[0].gekisou_ranges.as_ref().unwrap().iter().map(|r| r.raw_just).sum::<i32>()
    };
    assert!(raw(&active) > raw(&inactive), "4004 must increase realized Just count");
    assert!(
        active.results[0].best_order.as_ref().unwrap().score > inactive.results[0].best_order.as_ref().unwrap().score
    );
    request.execution = ournotes_search::types::Execution::Live {
        score_id: SCORE_ID,
        gekisou: true,
        play: PlayPolicy::Accuracy(ournotes_sim::live::model::Accuracy { great_fraction: 0.0, just_fraction: 0.5 }),
    };
    assert_eq!(
        engine::recommend(&data, &roster, &request).unwrap().results,
        engine::recommend(&inactive_data, &roster, &request).unwrap().results,
        "4004 remains a no-op for declared accuracy"
    );
}

#[test]
fn arena_oversubscribed_snap_domain_is_empty_and_rank_wire_is_strict() {
    let (data, roster, mut request) = inputs(6);
    request.constraints.include_snaps = (1..=6).collect();
    let out = engine::recommend(&data, &roster, &request).unwrap();
    assert!(out.results.is_empty());
    for percents in [json!([1, 2]), json!([1, 2, 3, 4]), json!([1.5, 2, 3])] {
        let mut value = joint_request_json("free", true, json!({"kind":"score"}));
        value["context"]["gekisouRankPercents"] = percents;
        assert!(serde_json::from_value::<RecommendationRequest>(value).is_err());
    }
}
