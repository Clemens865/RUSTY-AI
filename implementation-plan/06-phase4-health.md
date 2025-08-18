# Phase 4: Health & Wellness Suite Implementation

## Overview

Phase 4 introduces comprehensive health and wellness management capabilities, transforming the Personal AI Assistant into a health companion that monitors, analyzes, and provides personalized recommendations for physical and mental well-being.

## Feature Scope

### Core Health Features
1. **Health Data Aggregation**: Integration with fitness trackers, health apps, and medical devices
2. **Fitness Tracking**: Activity monitoring, workout planning, and progress tracking
3. **Nutrition Management**: Meal planning, calorie tracking, and nutritional analysis
4. **Sleep Analysis**: Sleep pattern monitoring and optimization recommendations
5. **Mental Health Support**: Mood tracking, meditation guidance, and stress management
6. **Medical Records**: Secure storage and organization of health documents
7. **Health Insights**: AI-powered health trend analysis and recommendations

### Integration Points
- Apple Health, Google Fit, Fitbit, Samsung Health
- MyFitnessPal, Cronometer, Lose It!
- Sleep tracking devices (Oura, WHOOP, smart watches)
- Meditation apps (Headspace, Calm, Insight Timer)
- Medical record systems (Epic MyChart, Cerner)

## Architecture Implementation

### 1. Health Service Layer

```toml
# crates/health/Cargo.toml
[package]
name = "health"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { workspace = true }
serde = { workspace = true }
anyhow = { workspace = true }
chrono = { version = "0.4", features = ["serde"] }
uuid = { workspace = true }
reqwest = { version = "0.11", features = ["json"] }
sqlx = { version = "0.7", features = ["postgres", "chrono", "uuid"] }
fitbit-web-api = "0.3"
apple-health-kit = "0.2"  # Hypothetical crate
nutritionix = "0.1"       # Hypothetical nutrition API crate
```

### 2. Core Health Models

```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, NaiveDate, NaiveTime};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthProfile {
    pub user_id: Uuid,
    pub birth_date: NaiveDate,
    pub height_cm: f32,
    pub weight_kg: f32,
    pub biological_sex: BiologicalSex,
    pub activity_level: ActivityLevel,
    pub health_goals: Vec<HealthGoal>,
    pub medical_conditions: Vec<String>,
    pub medications: Vec<Medication>,
    pub allergies: Vec<String>,
    pub emergency_contacts: Vec<EmergencyContact>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BiologicalSex {
    Male,
    Female,
    Other,
    PreferNotToSay,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityLevel {
    Sedentary,      // Little to no exercise
    LightlyActive,  // Light exercise 1-3 days/week
    ModeratelyActive, // Moderate exercise 3-5 days/week
    VeryActive,     // Hard exercise 6-7 days/week
    ExtremelyActive, // Physical job + exercise
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthGoal {
    pub id: Uuid,
    pub goal_type: HealthGoalType,
    pub target_value: f32,
    pub current_value: f32,
    pub target_date: Option<NaiveDate>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthGoalType {
    WeightLoss { target_kg: f32 },
    WeightGain { target_kg: f32 },
    StepsPerDay { target_steps: u32 },
    CaloriesBurned { target_calories: u32 },
    WaterIntake { target_liters: f32 },
    SleepHours { target_hours: f32 },
    ExerciseMinutes { target_minutes: u32 },
    MoodImprovement,
    StressReduction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Medication {
    pub name: String,
    pub dosage: String,
    pub frequency: String,
    pub prescribing_doctor: Option<String>,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub reminders_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetric {
    pub id: Uuid,
    pub user_id: Uuid,
    pub metric_type: HealthMetricType,
    pub value: f32,
    pub unit: String,
    pub recorded_at: DateTime<Utc>,
    pub source: DataSource,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthMetricType {
    Weight,
    BodyFat,
    HeartRate,
    BloodPressure { systolic: f32, diastolic: f32 },
    BloodSugar,
    Steps,
    CaloriesBurned,
    CaloriesConsumed,
    WaterIntake,
    SleepHours,
    SleepQuality,
    MoodScore,
    StressLevel,
    Temperature,
    VO2Max,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataSource {
    Manual,
    FitnessTracker { device: String },
    SmartScale { device: String },
    HealthApp { app: String },
    MedicalDevice { device: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workout {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub workout_type: WorkoutType,
    pub duration_minutes: u32,
    pub calories_burned: Option<u32>,
    pub exercises: Vec<Exercise>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkoutType {
    Cardio,
    Strength,
    Flexibility,
    Sports,
    Yoga,
    Pilates,
    HIIT,
    Walking,
    Running,
    Cycling,
    Swimming,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exercise {
    pub name: String,
    pub sets: Vec<ExerciseSet>,
    pub muscle_groups: Vec<MuscleGroup>,
    pub equipment_needed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExerciseSet {
    pub reps: Option<u32>,
    pub weight_kg: Option<f32>,
    pub duration_seconds: Option<u32>,
    pub distance_meters: Option<f32>,
    pub rest_seconds: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MuscleGroup {
    Chest, Back, Shoulders, Arms, Abs, Legs, Glutes, Core, Cardio
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meal {
    pub id: Uuid,
    pub user_id: Uuid,
    pub meal_type: MealType,
    pub foods: Vec<FoodItem>,
    pub total_calories: u32,
    pub macros: MacroNutrients,
    pub consumed_at: DateTime<Utc>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MealType {
    Breakfast,
    Lunch,
    Dinner,
    Snack,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodItem {
    pub name: String,
    pub quantity: f32,
    pub unit: String,
    pub calories_per_unit: f32,
    pub macros_per_unit: MacroNutrients,
    pub micronutrients: Vec<Micronutrient>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroNutrients {
    pub protein_g: f32,
    pub carbs_g: f32,
    pub fat_g: f32,
    pub fiber_g: f32,
    pub sugar_g: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Micronutrient {
    pub name: String,
    pub amount: f32,
    pub unit: String,
    pub daily_value_percent: Option<f32>,
}
```

### 3. Health Data Aggregation Service

```rust
use anyhow::Result;
use std::collections::HashMap;

pub struct HealthDataService {
    storage: Arc<HealthStorage>,
    fitness_integrations: HashMap<String, Box<dyn FitnessIntegration>>,
    nutrition_service: Arc<NutritionService>,
    ai_service: Arc<AIService>,
}

impl HealthDataService {
    pub async fn sync_all_health_data(&self, user_id: Uuid) -> Result<HealthSyncResult> {
        let mut sync_result = HealthSyncResult::default();
        
        // Sync from all connected fitness trackers
        for (source, integration) in &self.fitness_integrations {
            match integration.sync_recent_data(user_id).await {
                Ok(data) => {
                    sync_result.successful_sources.push(source.clone());
                    
                    // Store fitness data
                    for metric in data.metrics {
                        self.storage.store_health_metric(&metric).await?;
                    }
                    
                    for workout in data.workouts {
                        self.storage.store_workout(&workout).await?;
                    }
                    
                    for sleep_record in data.sleep_records {
                        self.storage.store_sleep_record(&sleep_record).await?;
                    }
                }
                Err(e) => {
                    sync_result.failed_sources.push((source.clone(), e.to_string()));
                }
            }
        }
        
        Ok(sync_result)
    }
    
    pub async fn analyze_health_trends(&self, user_id: Uuid, days: u32) -> Result<HealthTrendAnalysis> {
        let end_date = Utc::now();
        let start_date = end_date - chrono::Duration::days(days as i64);
        
        // Gather health metrics for analysis
        let metrics = self.storage
            .get_health_metrics_by_date_range(user_id, start_date, end_date)
            .await?;
        
        let workouts = self.storage
            .get_workouts_by_date_range(user_id, start_date, end_date)
            .await?;
        
        let sleep_records = self.storage
            .get_sleep_records_by_date_range(user_id, start_date, end_date)
            .await?;
        
        // Group metrics by type for trend analysis
        let mut grouped_metrics: HashMap<HealthMetricType, Vec<&HealthMetric>> = HashMap::new();
        for metric in &metrics {
            grouped_metrics.entry(metric.metric_type.clone()).or_default().push(metric);
        }
        
        let mut trends = Vec::new();
        
        // Analyze trends for each metric type
        for (metric_type, metric_values) in grouped_metrics {
            if metric_values.len() < 3 {
                continue; // Need at least 3 data points for trend analysis
            }
            
            let trend = self.calculate_trend(&metric_values);
            trends.push(HealthTrend {
                metric_type,
                direction: trend.direction,
                magnitude: trend.magnitude,
                confidence: trend.confidence,
                insights: self.ai_service.generate_trend_insights(&trend).await?,
            });
        }
        
        // Analyze workout consistency
        let workout_trend = self.analyze_workout_consistency(&workouts);
        
        // Analyze sleep quality trends
        let sleep_trend = self.analyze_sleep_trends(&sleep_records);
        
        Ok(HealthTrendAnalysis {
            analysis_period_days: days,
            metric_trends: trends,
            workout_consistency: workout_trend,
            sleep_quality_trend: sleep_trend,
            overall_health_score: self.calculate_overall_health_score(&metrics, &workouts, &sleep_records),
            recommendations: self.ai_service.generate_health_recommendations(user_id, &metrics, &workouts).await?,
            generated_at: Utc::now(),
        })
    }
    
    pub async fn suggest_personalized_workout(&self, user_id: Uuid) -> Result<WorkoutPlan> {
        let profile = self.storage.get_health_profile(user_id).await?;
        let recent_workouts = self.storage.get_recent_workouts(user_id, 14).await?;
        let fitness_level = self.assess_fitness_level(user_id).await?;
        
        // AI-powered workout recommendation
        let workout_suggestion = self.ai_service.suggest_workout(
            &profile,
            &recent_workouts,
            fitness_level,
        ).await?;
        
        Ok(workout_suggestion)
    }
    
    pub async fn track_nutrition_goals(&self, user_id: Uuid) -> Result<NutritionReport> {
        let profile = self.storage.get_health_profile(user_id).await?;
        let daily_targets = self.calculate_daily_nutrition_targets(&profile);
        
        let today = Utc::now().date_naive();
        let meals_today = self.storage.get_meals_by_date(user_id, today).await?;
        
        let consumed = meals_today.iter().fold(MacroNutrients::default(), |acc, meal| {
            MacroNutrients {
                protein_g: acc.protein_g + meal.macros.protein_g,
                carbs_g: acc.carbs_g + meal.macros.carbs_g,
                fat_g: acc.fat_g + meal.macros.fat_g,
                fiber_g: acc.fiber_g + meal.macros.fiber_g,
                sugar_g: acc.sugar_g + meal.macros.sugar_g,
            }
        });
        
        let total_calories_consumed: u32 = meals_today.iter().map(|m| m.total_calories).sum();
        
        Ok(NutritionReport {
            date: today,
            calorie_target: daily_targets.calories,
            calories_consumed: total_calories_consumed,
            calories_remaining: daily_targets.calories.saturating_sub(total_calories_consumed),
            macro_targets: daily_targets.macros,
            macros_consumed: consumed,
            meal_suggestions: self.ai_service.suggest_meals_for_remaining_macros(
                &daily_targets.macros,
                &consumed,
            ).await?,
        })
    }
    
    fn calculate_trend(&self, metrics: &[&HealthMetric]) -> TrendAnalysis {
        // Simple linear regression for trend analysis
        let n = metrics.len() as f32;
        let x_values: Vec<f32> = (0..metrics.len()).map(|i| i as f32).collect();
        let y_values: Vec<f32> = metrics.iter().map(|m| m.value).collect();
        
        let x_mean = x_values.iter().sum::<f32>() / n;
        let y_mean = y_values.iter().sum::<f32>() / n;
        
        let numerator: f32 = x_values.iter().zip(y_values.iter())
            .map(|(x, y)| (x - x_mean) * (y - y_mean))
            .sum();
        
        let denominator: f32 = x_values.iter()
            .map(|x| (x - x_mean).powi(2))
            .sum();
        
        let slope = if denominator != 0.0 { numerator / denominator } else { 0.0 };
        
        TrendAnalysis {
            direction: if slope > 0.1 {
                TrendDirection::Increasing
            } else if slope < -0.1 {
                TrendDirection::Decreasing
            } else {
                TrendDirection::Stable
            },
            magnitude: slope.abs(),
            confidence: self.calculate_trend_confidence(&y_values, slope),
        }
    }
    
    fn calculate_daily_nutrition_targets(&self, profile: &HealthProfile) -> DailyNutritionTargets {
        // Calculate BMR using Mifflin-St Jeor equation
        let bmr = match profile.biological_sex {
            BiologicalSex::Male => {
                10.0 * profile.weight_kg + 6.25 * profile.height_cm - 5.0 * self.calculate_age(profile.birth_date) as f32 + 5.0
            }
            BiologicalSex::Female => {
                10.0 * profile.weight_kg + 6.25 * profile.height_cm - 5.0 * self.calculate_age(profile.birth_date) as f32 - 161.0
            }
            _ => {
                // Use average of male/female calculation
                let male_bmr = 10.0 * profile.weight_kg + 6.25 * profile.height_cm - 5.0 * self.calculate_age(profile.birth_date) as f32 + 5.0;
                let female_bmr = 10.0 * profile.weight_kg + 6.25 * profile.height_cm - 5.0 * self.calculate_age(profile.birth_date) as f32 - 161.0;
                (male_bmr + female_bmr) / 2.0
            }
        };
        
        // Apply activity level multiplier
        let tdee = bmr * match profile.activity_level {
            ActivityLevel::Sedentary => 1.2,
            ActivityLevel::LightlyActive => 1.375,
            ActivityLevel::ModeratelyActive => 1.55,
            ActivityLevel::VeryActive => 1.725,
            ActivityLevel::ExtremelyActive => 1.9,
        };
        
        // Standard macro distribution: 25% protein, 45% carbs, 30% fat
        DailyNutritionTargets {
            calories: tdee as u32,
            macros: MacroNutrients {
                protein_g: (tdee * 0.25 / 4.0), // 4 calories per gram of protein
                carbs_g: (tdee * 0.45 / 4.0),   // 4 calories per gram of carbs
                fat_g: (tdee * 0.30 / 9.0),     // 9 calories per gram of fat
                fiber_g: 25.0, // Recommended daily fiber
                sugar_g: tdee * 0.10 / 4.0, // Max 10% of calories from added sugar
            },
        }
    }
}

#[derive(Debug, Default)]
pub struct HealthSyncResult {
    pub successful_sources: Vec<String>,
    pub failed_sources: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct HealthTrendAnalysis {
    pub analysis_period_days: u32,
    pub metric_trends: Vec<HealthTrend>,
    pub workout_consistency: WorkoutConsistencyTrend,
    pub sleep_quality_trend: SleepQualityTrend,
    pub overall_health_score: f32,
    pub recommendations: Vec<HealthRecommendation>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct HealthTrend {
    pub metric_type: HealthMetricType,
    pub direction: TrendDirection,
    pub magnitude: f32,
    pub confidence: f32,
    pub insights: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
}

#[derive(Debug, Clone)]
pub struct DailyNutritionTargets {
    pub calories: u32,
    pub macros: MacroNutrients,
}
```

### 4. Mental Health and Wellness

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoodEntry {
    pub id: Uuid,
    pub user_id: Uuid,
    pub mood_score: i32, // 1-10 scale
    pub energy_level: i32, // 1-10 scale
    pub stress_level: i32, // 1-10 scale
    pub anxiety_level: i32, // 1-10 scale
    pub emotions: Vec<Emotion>,
    pub notes: Option<String>,
    pub triggers: Vec<String>,
    pub coping_strategies_used: Vec<String>,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Emotion {
    Happy, Sad, Angry, Anxious, Excited, Calm, Frustrated, 
    Grateful, Lonely, Confident, Overwhelmed, Content, Other(String)
}

pub struct MentalHealthService {
    storage: Arc<HealthStorage>,
    ai_service: Arc<AIService>,
    notification_service: Arc<NotificationService>,
}

impl MentalHealthService {
    pub async fn analyze_mood_patterns(&self, user_id: Uuid, days: u32) -> Result<MoodAnalysis> {
        let mood_entries = self.storage
            .get_mood_entries_by_date_range(
                user_id, 
                Utc::now() - chrono::Duration::days(days as i64), 
                Utc::now()
            )
            .await?;
        
        if mood_entries.is_empty() {
            return Ok(MoodAnalysis::empty());
        }
        
        let avg_mood = mood_entries.iter().map(|e| e.mood_score).sum::<i32>() as f32 / mood_entries.len() as f32;
        let avg_energy = mood_entries.iter().map(|e| e.energy_level).sum::<i32>() as f32 / mood_entries.len() as f32;
        let avg_stress = mood_entries.iter().map(|e| e.stress_level).sum::<i32>() as f32 / mood_entries.len() as f32;
        
        // Identify patterns and triggers
        let common_triggers = self.identify_common_triggers(&mood_entries);
        let effective_coping_strategies = self.analyze_coping_effectiveness(&mood_entries);
        
        // AI insights
        let insights = self.ai_service.analyze_mental_health_patterns(&mood_entries).await?;
        
        Ok(MoodAnalysis {
            period_days: days,
            average_mood: avg_mood,
            average_energy: avg_energy,
            average_stress: avg_stress,
            mood_trend: self.calculate_mood_trend(&mood_entries),
            common_triggers,
            effective_coping_strategies,
            insights,
            recommendations: self.generate_mental_health_recommendations(&mood_entries, avg_mood).await?,
        })
    }
    
    pub async fn suggest_wellness_activities(&self, user_id: Uuid, current_mood: &MoodEntry) -> Result<Vec<WellnessActivity>> {
        let mut activities = Vec::new();
        
        // Suggest activities based on current mood and stress level
        if current_mood.stress_level > 7 {
            activities.extend(vec![
                WellnessActivity {
                    name: "5-Minute Breathing Exercise".to_string(),
                    activity_type: WellnessActivityType::Breathing,
                    duration_minutes: 5,
                    description: "Deep breathing to reduce stress and anxiety".to_string(),
                    instructions: vec![
                        "Find a comfortable position".to_string(),
                        "Breathe in for 4 counts".to_string(),
                        "Hold for 4 counts".to_string(),
                        "Exhale for 6 counts".to_string(),
                        "Repeat for 5 minutes".to_string(),
                    ],
                },
                WellnessActivity {
                    name: "Progressive Muscle Relaxation".to_string(),
                    activity_type: WellnessActivityType::Relaxation,
                    duration_minutes: 15,
                    description: "Systematic tensing and relaxing of muscle groups".to_string(),
                    instructions: vec![
                        "Start with your toes and work up".to_string(),
                        "Tense each muscle group for 5 seconds".to_string(),
                        "Release and notice the relaxation".to_string(),
                        "Move to the next muscle group".to_string(),
                    ],
                },
            ]);
        }
        
        if current_mood.mood_score < 5 {
            activities.push(WellnessActivity {
                name: "Gratitude Practice".to_string(),
                activity_type: WellnessActivityType::Mindfulness,
                duration_minutes: 10,
                description: "Focus on positive aspects of your life".to_string(),
                instructions: vec![
                    "Write down 3 things you're grateful for today".to_string(),
                    "Reflect on why each item is meaningful".to_string(),
                    "Notice how focusing on gratitude affects your mood".to_string(),
                ],
            });
        }
        
        if current_mood.energy_level < 4 {
            activities.push(WellnessActivity {
                name: "Energizing Movement".to_string(),
                activity_type: WellnessActivityType::Movement,
                duration_minutes: 10,
                description: "Light physical activity to boost energy".to_string(),
                instructions: vec![
                    "Do some gentle stretches".to_string(),
                    "Take a short walk outdoors".to_string(),
                    "Do 10 jumping jacks or bodyweight squats".to_string(),
                    "Focus on how movement affects your energy".to_string(),
                ],
            });
        }
        
        Ok(activities)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WellnessActivity {
    pub name: String,
    pub activity_type: WellnessActivityType,
    pub duration_minutes: u32,
    pub description: String,
    pub instructions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WellnessActivityType {
    Breathing,
    Meditation,
    Mindfulness,
    Relaxation,
    Movement,
    Journaling,
    Gratitude,
    Visualization,
}
```

## API Endpoints

```rust
pub fn create_health_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/health/profile", get(get_health_profile).put(update_health_profile))
        .route("/api/v1/health/metrics", post(log_health_metric).get(get_health_metrics))
        .route("/api/v1/health/sync", post(sync_health_data))
        .route("/api/v1/health/trends", get(get_health_trends))
        .route("/api/v1/health/workouts", post(log_workout).get(get_workouts))
        .route("/api/v1/health/workouts/suggest", get(suggest_workout))
        .route("/api/v1/health/nutrition", post(log_meal).get(get_nutrition_report))
        .route("/api/v1/health/mood", post(log_mood).get(get_mood_analysis))
        .route("/api/v1/health/wellness/activities", get(suggest_wellness_activities))
        .route("/api/v1/health/dashboard", get(get_health_dashboard))
}

pub async fn get_health_dashboard(
    State(state): State<AppState>,
) -> Result<Json<HealthDashboard>, StatusCode> {
    let user_id = get_authenticated_user_id()?;
    
    // Gather today's data
    let today = Utc::now().date_naive();
    let metrics_today = state.health.storage.get_health_metrics_by_date(user_id, today).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let nutrition_report = state.health.data_service.track_nutrition_goals(user_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let recent_workouts = state.health.storage.get_recent_workouts(user_id, 7).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mood_entries = state.health.storage.get_recent_mood_entries(user_id, 7).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Calculate summary stats
    let steps_today = metrics_today.iter()
        .find(|m| matches!(m.metric_type, HealthMetricType::Steps))
        .map(|m| m.value as u32)
        .unwrap_or(0);
    
    let avg_mood = if !mood_entries.is_empty() {
        mood_entries.iter().map(|m| m.mood_score).sum::<i32>() as f32 / mood_entries.len() as f32
    } else {
        0.0
    };
    
    let dashboard = HealthDashboard {
        steps_today,
        calories_consumed_today: nutrition_report.calories_consumed,
        calories_remaining_today: nutrition_report.calories_remaining,
        workouts_this_week: recent_workouts.len(),
        average_mood_this_week: avg_mood,
        active_health_goals: state.health.storage.get_active_health_goals(user_id).await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.len(),
        next_medication_reminder: state.health.storage.get_next_medication_reminder(user_id).await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        health_score: state.health.data_service.calculate_overall_health_score_for_user(user_id).await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    };
    
    Ok(Json(dashboard))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthDashboard {
    pub steps_today: u32,
    pub calories_consumed_today: u32,
    pub calories_remaining_today: u32,
    pub workouts_this_week: usize,
    pub average_mood_this_week: f32,
    pub active_health_goals: usize,
    pub next_medication_reminder: Option<DateTime<Utc>>,
    pub health_score: f32,
}
```

## Privacy and Security

### Health Data Encryption

```rust
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};

pub struct HealthDataEncryption {
    key: LessSafeKey,
}

impl HealthDataEncryption {
    pub fn encrypt_health_metric(&self, metric: &HealthMetric) -> Result<EncryptedHealthMetric, EncryptionError> {
        let serialized = serde_json::to_vec(metric)?;
        let encrypted_data = self.encrypt_data(&serialized)?;
        
        Ok(EncryptedHealthMetric {
            id: metric.id,
            user_id: metric.user_id,
            encrypted_data,
            recorded_at: metric.recorded_at,
        })
    }
    
    pub fn decrypt_health_metric(&self, encrypted: &EncryptedHealthMetric) -> Result<HealthMetric, EncryptionError> {
        let decrypted_data = self.decrypt_data(&encrypted.encrypted_data)?;
        let metric: HealthMetric = serde_json::from_slice(&decrypted_data)?;
        Ok(metric)
    }
}

// HIPAA compliance measures
pub struct HIPAAComplianceService {
    audit_logger: AuditLogger,
    access_control: AccessControlService,
}

impl HIPAAComplianceService {
    pub async fn log_health_data_access(&self, user_id: Uuid, data_type: &str, action: &str) -> Result<()> {
        self.audit_logger.log(AuditEvent {
            user_id,
            data_type: data_type.to_string(),
            action: action.to_string(),
            timestamp: Utc::now(),
            ip_address: None, // Would be captured from request
            user_agent: None, // Would be captured from request
        }).await
    }
    
    pub async fn ensure_minimum_necessary_access(&self, user_id: Uuid, requested_data: &str) -> Result<bool> {
        // Implement minimum necessary standard
        // Only allow access to health data that's needed for the specific function
        self.access_control.validate_data_access(user_id, requested_data).await
    }
}
```

## Performance Considerations

### Data Aggregation Optimization

```rust
use tokio::time::{interval, Duration};

pub struct HealthDataAggregator {
    storage: Arc<HealthStorage>,
    cache: Arc<HealthDataCache>,
}

impl HealthDataAggregator {
    pub async fn start_aggregation_tasks(&self) {
        // Aggregate daily health summaries
        let storage = self.storage.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(3600)); // Every hour
            loop {
                interval.tick().await;
                if let Err(e) = storage.aggregate_daily_summaries().await {
                    tracing::error!("Failed to aggregate daily summaries: {}", e);
                }
            }
        });
        
        // Cleanup old detailed data (keep summaries)
        let storage = self.storage.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(86400)); // Daily
            loop {
                interval.tick().await;
                if let Err(e) = storage.cleanup_old_detailed_data(365).await { // Keep 1 year
                    tracing::error!("Failed to cleanup old health data: {}", e);
                }
            }
        });
    }
}
```

Phase 4 creates a comprehensive health and wellness platform that respects user privacy while providing personalized insights and recommendations. The modular design allows for easy integration with various health platforms and devices while maintaining HIPAA compliance standards.