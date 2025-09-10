use polars::prelude::{col, lit, AnyValue, DataFrame, IntoLazy, UniqueKeepStrategy};
use std::collections::{BTreeMap, HashSet};

pub enum Partition {
    Unpartitioned,
    Partitioned(BTreeMap<String, DataFrame>),
}

/// Partition the [`DataFrame`] by unique combination of tag values across multiple tags.
///
/// Values in Tag column MUST be in String format.
///
/// Returns all the sub-DataFrames for each unique combination of tag values as a [`BTreeMap`]
/// where the key is the string concatenation of key-value pairs of all given tags.
///
/// If no tags are provided, returns [`Partition::Unpartitioned`]
pub fn partition_by_tags(data_frame: DataFrame, tags: &[&str]) -> anyhow::Result<Partition> {
    // Check for duplicate tag names
    let mut unique_tags = HashSet::with_capacity(tags.len());
    for &tag in tags {
        if !unique_tags.insert(tag) {
            return Err(anyhow::anyhow!("Duplicate tag name found: {}", tag));
        }
    }
    // If no tags provided, return [`Partition::Unpartitioned`]
    if tags.is_empty() {
        return Ok(Partition::Unpartitioned);
    }
    // Get unique combinations of all tag values
    let tag_columns: Vec<String> = tags.iter().map(|&tag| tag.to_string()).collect();
    let selectors = data_frame
        .clone()
        .lazy()
        .select(tags.iter().map(|&tag| col(tag)).collect::<Vec<_>>())
        .unique(Some(tag_columns), UniqueKeepStrategy::Any)
        .collect()?;

    // Create a map to store the sub-DataFrames
    let mut partitioned = BTreeMap::new();

    // For each row in the selectors DataFrame, we have a unique combination of tag values
    let n_rows = selectors.height();
    for row_idx in 0..n_rows {
        // Build a filter expression for this specific combination of tag values
        let mut filter_expr = None;
        let mut key_parts = Vec::with_capacity(tags.len());

        for &tag in tags {
            // Get the value for this tag in the current row
            let tag_value = match selectors.column(tag)?.get(row_idx) {
                Ok(AnyValue::String(s)) => s.to_string(),
                Ok(AnyValue::StringOwned(s)) => s.into_string(),
                Ok(v) => {
                    // Skip if not a string value
                    log::warn!("In Tag Column {tag}, found non String value: {v:?}");
                    continue;
                }
                Err(e) => {
                    // Skip on error
                    log::error!("In Tag Column {tag}: {e}");
                    continue;
                }
            };

            key_parts.push(format!("{}={}", tag, tag_value));

            // Build a filter expression that combines all tag conditions
            let tag_filter = col(tag).eq(lit(tag_value));
            filter_expr = match filter_expr {
                None => Some(tag_filter),
                Some(expr) => Some(expr.and(tag_filter)),
            };
        }

        // Create a key that represents this unique combination of tag values
        let combination_key = key_parts.join(",");
        log::debug!("Partition for {}", combination_key);

        // Apply the filter to get rows matching this combination of tag values
        if let Some(filter) = filter_expr {
            let filtered = data_frame
                .clone()
                .lazy()
                .select([col("*")])
                .filter(filter)
                .collect()?;

            partitioned.insert(combination_key, filtered);
        } else {
            log::warn!("No rows found matching tag combination {}", combination_key);
        }
    }

    Ok(Partition::Partitioned(partitioned))
}

#[cfg(test)]
mod tests {
    use super::*;
    use polars::prelude::df;

    // Helper to create a test DataFrame with tags
    fn create_test_dataframe() -> DataFrame {
        df! [
            "tag1"  => ["a", "a", "b", "b", "a", "c"],
            "tag2"  => ["x", "y", "x", "y", "x", "z"],
            "value" => [1.,  2.,  3.,  4.,  5.,  6.],
            "numeric_tag" => [6, 5, 4, 3, 2, 1],
        ]
        .unwrap()
    }

    #[test]
    fn test_partition_with_no_tags() -> anyhow::Result<()> {
        let df = create_test_dataframe();
        let partitioned = partition_by_tags(df.clone(), &[])?;
        // The group should contain all rows from the original DataFrame
        let Partition::Unpartitioned = partitioned else {
            panic!("Expected Unpartitioned DataFrame");
        };
        Ok(())
    }

    #[test]
    fn test_partition_with_nonexistent_tag() -> anyhow::Result<()> {
        let df = create_test_dataframe();
        // Test partition with a nonexistent tag should result in an error
        let result = partition_by_tags(df.clone(), &["nonexistent"]);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_partition_with_duplicate_tags() -> anyhow::Result<()> {
        let df = create_test_dataframe();
        // Test partition with duplicate tags should result in an error
        let result = partition_by_tags(df.clone(), &["tag1", "tag1"]);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_partition_with_empty_tag() -> anyhow::Result<()> {
        let df = create_test_dataframe();
        // Test partition with an empty tag should result in an error
        let result = partition_by_tags(df.clone(), &["tag1", ""]);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_partition_by_single_tag() -> anyhow::Result<()> {
        let df = create_test_dataframe();
        let partition = partition_by_tags(df.clone(), &["tag1"])?;
        let Partition::Partitioned(partitioned) = partition else {
            panic!("Expected Partitioned DataFrame");
        };
        // Should have 3 groups: a, b, c
        assert_eq!(partitioned.len(), 3);
        // Check group "a" has 3 rows
        assert!(partitioned.contains_key("tag1=a"));
        assert_eq!(partitioned["tag1=a"].height(), 3);
        // Check group "b" has 2 rows
        assert!(partitioned.contains_key("tag1=b"));
        assert_eq!(partitioned["tag1=b"].height(), 2);
        // Check group "c" has 1 row
        assert!(partitioned.contains_key("tag1=c"));
        assert_eq!(partitioned["tag1=c"].height(), 1);

        // Check that the "a" group contains the correct values
        let a_group = &partitioned["tag1=a"];
        let values: Vec<f64> = a_group
            .column("value")?
            .f64()?
            .into_iter()
            .map(|v| v.unwrap())
            .collect();
        assert_eq!(values, vec![1., 2., 5.]);

        // Check that the "b" group contains the correct values
        let b_group = &partitioned["tag1=b"];
        let values: Vec<f64> = b_group
            .column("value")?
            .f64()?
            .into_iter()
            .map(|v| v.unwrap())
            .collect();
        assert_eq!(values, vec![3., 4.]);

        Ok(())
    }

    #[test]
    fn test_partition_by_two_tags() -> anyhow::Result<()> {
        let df = create_test_dataframe();
        let partition = partition_by_tags(df.clone(), &["tag1", "tag2"])?;
        let Partition::Partitioned(partitioned) = partition else {
            panic!("Expected Partitioned DataFrame");
        };
        // Should have 5 combinations: (a,x), (a,y), (b,x), (b,y), (c,z)
        assert_eq!(partitioned.len(), 5);
        // Check specific combinations
        assert!(partitioned.contains_key("tag1=a,tag2=x"));
        assert_eq!(partitioned["tag1=a,tag2=x"].height(), 2); // 2 rows with a,x

        assert!(partitioned.contains_key("tag1=a,tag2=y"));
        assert_eq!(partitioned["tag1=a,tag2=y"].height(), 1); // 1 row with a,y

        assert!(partitioned.contains_key("tag1=b,tag2=x"));
        assert_eq!(partitioned["tag1=b,tag2=x"].height(), 1); // 1 row with b,x

        assert!(partitioned.contains_key("tag1=b,tag2=y"));
        assert_eq!(partitioned["tag1=b,tag2=y"].height(), 1); // 1 row with b,y

        assert!(partitioned.contains_key("tag1=c,tag2=z"));
        assert_eq!(partitioned["tag1=c,tag2=z"].height(), 1); // 1 row with c,z

        Ok(())
    }

    #[test]
    fn test_partition_with_numerical_tag() -> anyhow::Result<()> {
        let df = create_test_dataframe();
        // Test partition with one string tag and one numeric tag
        let partition = partition_by_tags(df.clone(), &["tag1", "numeric_tag"])?;
        let Partition::Partitioned(partitioned) = partition else {
            panic!("Expected Partitioned DataFrame");
        };

        // Should have 3 combinations: a,b,c
        assert_eq!(partitioned.len(), 3);

        // Check group "a" has 3 rows
        assert!(partitioned.contains_key("tag1=a"));
        assert_eq!(partitioned["tag1=a"].height(), 3);
        // Check group "b" has 2 rows
        assert!(partitioned.contains_key("tag1=b"));
        assert_eq!(partitioned["tag1=b"].height(), 2);
        // Check group "c" has 1 row
        assert!(partitioned.contains_key("tag1=c"));
        assert_eq!(partitioned["tag1=c"].height(), 1);

        Ok(())
    }
}
