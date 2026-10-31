import pandas as pd
import pytest

from trainlab.data import splits as S


def _df(n=100):
    return pd.DataFrame({"image_id": [f"i{k}" for k in range(n)], "label": [k % 5 for k in range(n)],
                         "group_id": [f"g{k // 2}" for k in range(n)], "sha256": [f"h{k}" for k in range(n)]})


def test_groups_never_straddle_splits_and_deterministic():
    df = _df()
    fr = {"train": 0.7, "val": 0.15, "test": 0.15}
    s1 = S.stratified_group_split(df, "label", "group_id", fr, seed=1)
    s2 = S.stratified_group_split(df, "label", "group_id", fr, seed=1)
    assert s1.equals(s2)
    assert (df.assign(split=s1).groupby("group_id")["split"].nunique() == 1).all()
    assert set(s1) == set(fr) and s1.notna().all()
    assert abs((s1 == "train").mean() - 0.7) < 0.1


def test_leakage_detected_on_shared_hash_and_clean_otherwise():
    df = _df()
    a, b = df.iloc[:10], df.iloc[10:20].copy()
    assert S.leakage_report({"a": a, "b": b})["ok"]
    b.iloc[0, b.columns.get_loc("sha256")] = a.iloc[0]["sha256"]
    rep = S.leakage_report({"a": a, "b": b})
    assert not rep["ok"] and rep["overlaps"][0]["column"] == "sha256"
    with pytest.raises(RuntimeError):
        S.assert_no_leakage({"a": a, "b": b})


def test_duplicate_groups_merge_exact_duplicates():
    df = pd.DataFrame({"image_id": ["a", "b", "c"], "sha256": ["x", "x", "y"]})
    g = S.assign_duplicate_groups(df, ("sha256",))
    assert g[0] == g[1] != g[2]


def test_bad_fractions_rejected():
    with pytest.raises(ValueError):
        S.stratified_group_split(_df(), "label", "group_id", {"a": 0.5, "b": 0.6})
