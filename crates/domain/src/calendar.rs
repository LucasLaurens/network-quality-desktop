//! Days since 1970-01-01 turned into calendar dates, in the proleptic Gregorian calendar.

/// Days from 0000-03-01 to 1970-01-01.
const DAYS_TO_EPOCH: i64 = 719_468;
const DAYS_PER_ERA: i64 = 146_097;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CivilDate {
    pub year: i64,
    pub month: i64,
    pub day: i64,
}

impl CivilDate {
    /// H. Hinnant's `civil_from_days`. O(1).
    #[must_use]
    pub const fn from_days(days: i64) -> Self {
        let shifted = days + DAYS_TO_EPOCH;
        let era = shifted.div_euclid(DAYS_PER_ERA);
        let day_of_era = shifted.rem_euclid(DAYS_PER_ERA);
        let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524
            - day_of_era / (DAYS_PER_ERA - 1))
            / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        // Months counted from March, so that February comes last.
        let month_from_march = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
        let month = if month_from_march < 10 {
            month_from_march + 3
        } else {
            month_from_march - 9
        };
        let march_year = year_of_era + era * 400;
        Self {
            year: if month <= 2 {
                march_year + 1
            } else {
                march_year
            },
            month,
            day,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_date_of_a_day() {
        // Act & Assert
        assert_eq!(
            CivilDate::from_days(0),
            CivilDate {
                year: 1970,
                month: 1,
                day: 1
            }
        );
        assert_eq!(
            CivilDate::from_days(19_782),
            CivilDate {
                year: 2024,
                month: 2,
                day: 29
            }
        );
        assert_eq!(
            CivilDate::from_days(-1),
            CivilDate {
                year: 1969,
                month: 12,
                day: 31
            }
        );
    }
}
