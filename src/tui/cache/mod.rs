mod policy;
mod remote;
mod status;
mod store;

pub use policy::{Age, RefreshPolicy};
pub use remote::{Phase, Remote, Stale};
pub use status::{Access, CacheStatus};
pub use store::Cache;

#[cfg(test)]
mod tests {
    use super::*;

    const POLICY: RefreshPolicy = RefreshPolicy::new(60, 600);

    #[test]
    fn age_tiers_are_bounded_by_the_policy() {
        assert_eq!(POLICY.classify(0), Age::Fresh);
        assert_eq!(POLICY.classify(59), Age::Fresh);
        assert_eq!(POLICY.classify(60), Age::Stale);
        assert_eq!(POLICY.classify(599), Age::Stale);
        assert_eq!(POLICY.classify(600), Age::Cold);
    }

    #[test]
    fn access_covers_presence_freshness_and_in_flight() {
        let empty: Remote<i32> = Remote::default();
        assert_eq!(empty.access(0, &POLICY), Access::Load);

        let fresh = Remote::ready(1, 100);
        assert_eq!(fresh.access(100, &POLICY), Access::Skip);
        assert_eq!(fresh.access(100 + 120, &POLICY), Access::Revalidate);
        assert_eq!(fresh.access(100 + 700, &POLICY), Access::Bust);

        let mut loading: Remote<i32> = Remote::default();
        loading.begin();
        assert_eq!(loading.access(10_000, &POLICY), Access::Skip);
    }

    #[test]
    fn begin_is_loading_without_a_value_and_revalidating_with_one() {
        let mut cell: Remote<i32> = Remote::default();
        cell.begin();
        assert_eq!(*cell.status(), CacheStatus::Loading);

        cell.set(7, 100);
        cell.begin();
        assert_eq!(*cell.status(), CacheStatus::Revalidating);
    }

    #[test]
    fn fail_keeps_the_value_and_bust_drops_it() {
        let mut cell = Remote::ready(9, 100);
        cell.fail("boom".into());
        assert_eq!(cell.value(), Some(&9));
        assert_eq!(*cell.status(), CacheStatus::Failed("boom".into()));

        cell.bust();
        assert_eq!(cell.value(), None);
        assert_eq!(*cell.status(), CacheStatus::Idle);
    }

    #[test]
    fn phase_reports_the_view_for_each_value_status_pairing() {
        let missing: Remote<i32> = Remote::default();
        assert_eq!(missing.phase(), Phase::Missing);

        let mut loading: Remote<i32> = Remote::default();
        loading.begin();
        assert_eq!(loading.phase(), Phase::Loading);

        let ready = Remote::ready(3, 100);
        assert_eq!(ready.phase(), Phase::Ready);

        let mut failed: Remote<i32> = Remote::default();
        failed.fail("nope".into());
        assert_eq!(failed.phase(), Phase::Failed);
    }

    #[test]
    fn invalidate_all_marks_every_cell_stale() {
        let mut cache: Cache<&str, Remote<i32>> = Cache::default();
        cache.insert("a", Remote::ready(1, 5_000));
        cache.insert("b", Remote::ready(2, 5_000));
        cache.invalidate_all();
        assert!(cache.get(&"a").unwrap().access(5_000, &POLICY) != Access::Skip);
    }
}
