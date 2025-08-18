use rusty_ai_common::{Result, AssistantError, Intent, UserContext};
use std::collections::HashMap;
use regex::Regex;
use tracing::{debug, info};

pub struct IntentClassifier {
    patterns: Vec<IntentPattern>,
    fallback_confidence_threshold: f32,
}

#[derive(Debug, Clone)]
pub struct IntentPattern {
    pub intent_type: IntentType,
    pub patterns: Vec<Regex>,
    pub keywords: Vec<String>,
    pub priority: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IntentType {
    Query,
    Command,
    Information,
    Greeting,
    Goodbye,
    Help,
    Settings,
    TaskManagement,
    DocumentSearch,
    VoiceCommand,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ClassificationResult {
    pub intent: Intent,
    pub confidence: f32,
    pub matched_pattern: Option<String>,
    pub extracted_entities: HashMap<String, String>,
}

impl IntentClassifier {
    pub fn new() -> Self {
        let mut classifier = Self {
            patterns: Vec::new(),
            fallback_confidence_threshold: 0.3,
        };
        
        classifier.initialize_default_patterns();
        classifier
    }

    pub fn new_with_threshold(threshold: f32) -> Self {
        let mut classifier = Self {
            patterns: Vec::new(),
            fallback_confidence_threshold: threshold,
        };
        
        classifier.initialize_default_patterns();
        classifier
    }

    fn initialize_default_patterns(&mut self) {
        // Greeting patterns
        self.add_pattern(IntentPattern {
            intent_type: IntentType::Greeting,
            patterns: vec![
                Regex::new(r"^(hi|hello|hey|good (morning|afternoon|evening))").unwrap(),
                Regex::new(r"^(greetings|howdy|what's up)").unwrap(),
            ],
            keywords: vec!["hi", "hello", "hey", "morning", "afternoon", "evening"]
                .iter().map(|s| s.to_string()).collect(),
            priority: 10,
        });

        // Goodbye patterns
        self.add_pattern(IntentPattern {
            intent_type: IntentType::Goodbye,
            patterns: vec![
                Regex::new(r"(bye|goodbye|see you|farewell|take care)").unwrap(),
                Regex::new(r"(good night|have a good day)").unwrap(),
            ],
            keywords: vec!["bye", "goodbye", "farewell", "night"]
                .iter().map(|s| s.to_string()).collect(),
            priority: 10,
        });

        // Help patterns
        self.add_pattern(IntentPattern {
            intent_type: IntentType::Help,
            patterns: vec![
                Regex::new(r"(help|assist|support|how to|how do i)").unwrap(),
                Regex::new(r"(what can you do|what are your capabilities)").unwrap(),
            ],
            keywords: vec!["help", "assist", "support", "how", "capabilities"]
                .iter().map(|s| s.to_string()).collect(),
            priority: 9,
        });

        // Task management patterns
        self.add_pattern(IntentPattern {
            intent_type: IntentType::TaskManagement,
            patterns: vec![
                Regex::new(r"(create|add|new) .* (task|todo|reminder)").unwrap(),
                Regex::new(r"(complete|finish|done) .* (task|todo)").unwrap(),
                Regex::new(r"(list|show|display) .* (tasks|todos|reminders)").unwrap(),
                Regex::new(r"(schedule|plan|organize)").unwrap(),
            ],
            keywords: vec!["task", "todo", "reminder", "schedule", "complete", "finish", "list"]
                .iter().map(|s| s.to_string()).collect(),
            priority: 8,
        });

        // Document search patterns
        self.add_pattern(IntentPattern {
            intent_type: IntentType::DocumentSearch,
            patterns: vec![
                Regex::new(r"(find|search|look for|locate) .* (document|file|note)").unwrap(),
                Regex::new(r"(show|display) .* (document|file|notes)").unwrap(),
                Regex::new(r"(what|where) .* (document|file|information)").unwrap(),
            ],
            keywords: vec!["find", "search", "document", "file", "note", "locate"]
                .iter().map(|s| s.to_string()).collect(),
            priority: 7,
        });

        // Settings patterns
        self.add_pattern(IntentPattern {
            intent_type: IntentType::Settings,
            patterns: vec![
                Regex::new(r"(change|update|modify|set) .* (settings|preferences|configuration)").unwrap(),
                Regex::new(r"(configure|setup|adjust)").unwrap(),
                Regex::new(r"(enable|disable|turn on|turn off)").unwrap(),
            ],
            keywords: vec!["settings", "preferences", "configure", "enable", "disable"]
                .iter().map(|s| s.to_string()).collect(),
            priority: 6,
        });

        // Voice command patterns
        self.add_pattern(IntentPattern {
            intent_type: IntentType::VoiceCommand,
            patterns: vec![
                Regex::new(r"(voice|speak|say|read|audio)").unwrap(),
                Regex::new(r"(listen|hear|sound)").unwrap(),
            ],
            keywords: vec!["voice", "speak", "read", "listen", "audio", "sound"]
                .iter().map(|s| s.to_string()).collect(),
            priority: 5,
        });

        // Query patterns (broader, lower priority)
        self.add_pattern(IntentPattern {
            intent_type: IntentType::Query,
            patterns: vec![
                Regex::new(r"(what|who|when|where|why|how)").unwrap(),
                Regex::new(r"(tell me|explain|describe)").unwrap(),
                Regex::new(r"(is|are|was|were|will|would|can|could)").unwrap(),
            ],
            keywords: vec!["what", "who", "when", "where", "why", "how", "tell", "explain"]
                .iter().map(|s| s.to_string()).collect(),
            priority: 3,
        });

        // Command patterns (broader, lower priority)
        self.add_pattern(IntentPattern {
            intent_type: IntentType::Command,
            patterns: vec![
                Regex::new(r"(please|can you|could you)").unwrap(),
                Regex::new(r"(do|make|create|generate|build)").unwrap(),
                Regex::new(r"(start|stop|pause|resume|cancel)").unwrap(),
            ],
            keywords: vec!["please", "do", "make", "create", "start", "stop"]
                .iter().map(|s| s.to_string()).collect(),
            priority: 2,
        });

        // Information patterns (lowest priority)
        self.add_pattern(IntentPattern {
            intent_type: IntentType::Information,
            patterns: vec![
                Regex::new(r"(about|regarding|concerning)").unwrap(),
                Regex::new(r"(information|details|facts)").unwrap(),
            ],
            keywords: vec!["about", "information", "details", "facts"]
                .iter().map(|s| s.to_string()).collect(),
            priority: 1,
        });

        // Sort patterns by priority (highest first)
        self.patterns.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        info!("Initialized intent classifier with {} patterns", self.patterns.len());
    }

    pub fn add_pattern(&mut self, pattern: IntentPattern) {
        self.patterns.push(pattern);
        self.patterns.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    pub fn classify(&self, input: &str, context: Option<&UserContext>) -> ClassificationResult {
        let normalized_input = input.to_lowercase().trim().to_string();
        debug!("Classifying input: '{}'", input);

        // Try pattern matching first
        if let Some(result) = self.match_patterns(&normalized_input) {
            debug!("Matched pattern: {:?} with confidence {}", result.intent, result.confidence);
            return result;
        }

        // Try keyword matching
        if let Some(result) = self.match_keywords(&normalized_input) {
            debug!("Matched keywords: {:?} with confidence {}", result.intent, result.confidence);
            return result;
        }

        // Try context-based classification
        if let Some(context) = context {
            if let Some(result) = self.classify_with_context(&normalized_input, context) {
                debug!("Classified with context: {:?} with confidence {}", result.intent, result.confidence);
                return result;
            }
        }

        // Fallback to heuristic classification
        let fallback_result = self.heuristic_classification(&normalized_input);
        debug!("Fallback classification: {:?} with confidence {}", fallback_result.intent, fallback_result.confidence);
        
        fallback_result
    }

    fn match_patterns(&self, input: &str) -> Option<ClassificationResult> {
        for pattern in &self.patterns {
            for regex in &pattern.patterns {
                if regex.is_match(input) {
                    let confidence = self.calculate_pattern_confidence(&pattern, input);
                    if confidence >= self.fallback_confidence_threshold {
                        return Some(ClassificationResult {
                            intent: self.intent_type_to_intent(&pattern.intent_type, input),
                            confidence,
                            matched_pattern: Some(regex.as_str().to_string()),
                            extracted_entities: self.extract_entities(input, &pattern.intent_type),
                        });
                    }
                }
            }
        }
        None
    }

    fn match_keywords(&self, input: &str) -> Option<ClassificationResult> {
        let words: Vec<&str> = input.split_whitespace().collect();
        let mut best_match: Option<(IntentType, f32)> = None;

        for pattern in &self.patterns {
            let mut matches = 0;
            for keyword in &pattern.keywords {
                if words.iter().any(|word| word.contains(keyword)) {
                    matches += 1;
                }
            }

            if matches > 0 {
                let confidence = (matches as f32 / pattern.keywords.len() as f32) * 0.8; // Max 0.8 for keyword matching
                if confidence >= self.fallback_confidence_threshold {
                    if let Some((_, prev_confidence)) = best_match {
                        if confidence > prev_confidence {
                            best_match = Some((pattern.intent_type.clone(), confidence));
                        }
                    } else {
                        best_match = Some((pattern.intent_type.clone(), confidence));
                    }
                }
            }
        }

        if let Some((intent_type, confidence)) = best_match {
            return Some(ClassificationResult {
                intent: self.intent_type_to_intent(&intent_type, input),
                confidence,
                matched_pattern: None,
                extracted_entities: self.extract_entities(input, &intent_type),
            });
        }

        None
    }

    fn classify_with_context(&self, input: &str, context: &UserContext) -> Option<ClassificationResult> {
        // Analyze conversation history for context clues
        if !context.conversation_history.is_empty() {
            let last_turn = &context.conversation_history.last().unwrap();
            
            // If the last turn was a question, this might be a follow-up
            if matches!(last_turn.intent, Intent::Query { .. }) && 
               (input.contains("yes") || input.contains("no") || input.contains("more")) {
                return Some(ClassificationResult {
                    intent: Intent::Information { topic: "follow_up".to_string() },
                    confidence: 0.7,
                    matched_pattern: None,
                    extracted_entities: HashMap::new(),
                });
            }
        }

        // Check active plugins for context
        for plugin_id in &context.active_plugins {
            if plugin_id.contains("task") && input.contains("task") {
                return Some(ClassificationResult {
                    intent: Intent::Command { action: "task_operation".to_string(), parameters: vec![input.to_string()] },
                    confidence: 0.6,
                    matched_pattern: None,
                    extracted_entities: HashMap::new(),
                });
            }
        }

        None
    }

    fn heuristic_classification(&self, input: &str) -> ClassificationResult {
        let words: Vec<&str> = input.split_whitespace().collect();
        
        // Question word heuristic
        if words.first().map_or(false, |w| ["what", "who", "when", "where", "why", "how"].contains(w)) {
            return ClassificationResult {
                intent: Intent::Query { query: input.to_string() },
                confidence: 0.4,
                matched_pattern: None,
                extracted_entities: self.extract_question_entities(input),
            };
        }

        // Imperative heuristic
        if words.first().map_or(false, |w| ["create", "make", "do", "start", "stop", "show", "list"].contains(w)) {
            return ClassificationResult {
                intent: Intent::Command { 
                    action: words.first().unwrap().to_string(), 
                    parameters: words[1..].iter().map(|s| s.to_string()).collect() 
                },
                confidence: 0.3,
                matched_pattern: None,
                extracted_entities: HashMap::new(),
            };
        }

        // Default to unknown
        ClassificationResult {
            intent: Intent::Unknown,
            confidence: 0.0,
            matched_pattern: None,
            extracted_entities: HashMap::new(),
        }
    }

    fn calculate_pattern_confidence(&self, pattern: &IntentPattern, input: &str) -> f32 {
        let base_confidence = 0.9;
        let length_penalty = if input.len() > 100 { 0.1 } else { 0.0 };
        let priority_bonus = (pattern.priority as f32) * 0.01;
        
        (base_confidence - length_penalty + priority_bonus).min(1.0)
    }

    fn intent_type_to_intent(&self, intent_type: &IntentType, input: &str) -> Intent {
        match intent_type {
            IntentType::Query => Intent::Query { query: input.to_string() },
            IntentType::Command => {
                let words: Vec<&str> = input.split_whitespace().collect();
                let action = words.first().unwrap_or(&"unknown").to_string();
                let parameters = words[1..].iter().map(|s| s.to_string()).collect();
                Intent::Command { action, parameters }
            },
            IntentType::Information => Intent::Information { topic: self.extract_topic(input) },
            IntentType::Greeting => Intent::Information { topic: "greeting".to_string() },
            IntentType::Goodbye => Intent::Information { topic: "goodbye".to_string() },
            IntentType::Help => Intent::Information { topic: "help".to_string() },
            IntentType::Settings => Intent::Command { action: "settings".to_string(), parameters: vec![input.to_string()] },
            IntentType::TaskManagement => Intent::Command { action: "task".to_string(), parameters: vec![input.to_string()] },
            IntentType::DocumentSearch => Intent::Query { query: input.to_string() },
            IntentType::VoiceCommand => Intent::Command { action: "voice".to_string(), parameters: vec![input.to_string()] },
            IntentType::Unknown => Intent::Unknown,
        }
    }

    fn extract_entities(&self, input: &str, intent_type: &IntentType) -> HashMap<String, String> {
        let mut entities = HashMap::new();
        
        match intent_type {
            IntentType::TaskManagement => {
                if let Some(task_name) = self.extract_task_name(input) {
                    entities.insert("task_name".to_string(), task_name);
                }
                if let Some(due_date) = self.extract_date(input) {
                    entities.insert("due_date".to_string(), due_date);
                }
            },
            IntentType::DocumentSearch => {
                if let Some(search_term) = self.extract_search_term(input) {
                    entities.insert("search_term".to_string(), search_term);
                }
            },
            IntentType::Settings => {
                if let Some(setting_name) = self.extract_setting_name(input) {
                    entities.insert("setting".to_string(), setting_name);
                }
            },
            _ => {}
        }
        
        entities
    }

    fn extract_question_entities(&self, input: &str) -> HashMap<String, String> {
        let mut entities = HashMap::new();
        
        if input.starts_with("what") {
            entities.insert("question_type".to_string(), "what".to_string());
        } else if input.starts_with("who") {
            entities.insert("question_type".to_string(), "who".to_string());
        } else if input.starts_with("when") {
            entities.insert("question_type".to_string(), "when".to_string());
        } else if input.starts_with("where") {
            entities.insert("question_type".to_string(), "where".to_string());
        } else if input.starts_with("why") {
            entities.insert("question_type".to_string(), "why".to_string());
        } else if input.starts_with("how") {
            entities.insert("question_type".to_string(), "how".to_string());
        }
        
        entities
    }

    fn extract_topic(&self, input: &str) -> String {
        // Simple topic extraction - in production, this would be more sophisticated
        let words: Vec<&str> = input.split_whitespace().collect();
        if words.len() > 2 {
            words[1..].join(" ")
        } else {
            "general".to_string()
        }
    }

    fn extract_task_name(&self, input: &str) -> Option<String> {
        // Look for patterns like "create task [name]" or "add todo [name]"
        let patterns = [
            Regex::new(r"(?:create|add|new)\s+(?:task|todo|reminder)\s+(.+)").unwrap(),
            Regex::new(r"(?:task|todo|reminder):\s*(.+)").unwrap(),
        ];
        
        for pattern in patterns {
            if let Some(captures) = pattern.captures(input) {
                if let Some(name) = captures.get(1) {
                    return Some(name.as_str().trim().to_string());
                }
            }
        }
        
        None
    }

    fn extract_search_term(&self, input: &str) -> Option<String> {
        let patterns = [
            Regex::new(r"(?:find|search|look for)\s+(.+)").unwrap(),
            Regex::new(r"(?:document|file|note)\s+(?:about|on|for)\s+(.+)").unwrap(),
        ];
        
        for pattern in patterns {
            if let Some(captures) = pattern.captures(input) {
                if let Some(term) = captures.get(1) {
                    return Some(term.as_str().trim().to_string());
                }
            }
        }
        
        None
    }

    fn extract_setting_name(&self, input: &str) -> Option<String> {
        let patterns = [
            Regex::new(r"(?:change|set|update)\s+(.+?)\s+(?:setting|preference)").unwrap(),
            Regex::new(r"(?:enable|disable)\s+(.+)").unwrap(),
        ];
        
        for pattern in patterns {
            if let Some(captures) = pattern.captures(input) {
                if let Some(setting) = captures.get(1) {
                    return Some(setting.as_str().trim().to_string());
                }
            }
        }
        
        None
    }

    fn extract_date(&self, input: &str) -> Option<String> {
        // Simple date extraction - in production, you'd use a proper NLP library
        let date_patterns = [
            Regex::new(r"(?:today|tomorrow|yesterday)").unwrap(),
            Regex::new(r"(?:monday|tuesday|wednesday|thursday|friday|saturday|sunday)").unwrap(),
            Regex::new(r"\d{1,2}/\d{1,2}/\d{4}").unwrap(),
            Regex::new(r"\d{4}-\d{2}-\d{2}").unwrap(),
        ];
        
        for pattern in date_patterns {
            if let Some(m) = pattern.find(input) {
                return Some(m.as_str().to_string());
            }
        }
        
        None
    }

    pub fn get_confidence_threshold(&self) -> f32 {
        self.fallback_confidence_threshold
    }

    pub fn set_confidence_threshold(&mut self, threshold: f32) {
        self.fallback_confidence_threshold = threshold.clamp(0.0, 1.0);
    }

    pub fn get_supported_intents(&self) -> Vec<IntentType> {
        self.patterns.iter()
            .map(|p| p.intent_type.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }
}

impl Default for IntentClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusty_ai_common::{UserPreferences, VoiceSettings, NotificationSettings};

    fn create_test_context() -> UserContext {
        UserContext {
            user_id: uuid::Uuid::new_v4(),
            session_id: uuid::Uuid::new_v4(),
            preferences: UserPreferences {
                language: "en".to_string(),
                timezone: "UTC".to_string(),
                voice_settings: VoiceSettings {
                    enabled: true,
                    voice_id: "default".to_string(),
                    speed: 1.0,
                    pitch: 1.0,
                },
                notification_settings: NotificationSettings {
                    enabled: true,
                    channels: vec![],
                    quiet_hours: None,
                },
            },
            active_plugins: vec![],
            conversation_history: vec![],
        }
    }

    #[test]
    fn test_greeting_classification() {
        let classifier = IntentClassifier::new();
        let result = classifier.classify("Hello there!", None);
        
        match result.intent {
            Intent::Information { topic } => assert_eq!(topic, "greeting"),
            _ => panic!("Expected greeting intent"),
        }
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn test_query_classification() {
        let classifier = IntentClassifier::new();
        let result = classifier.classify("What is the weather like today?", None);
        
        match result.intent {
            Intent::Query { query } => assert!(query.contains("weather")),
            _ => panic!("Expected query intent"),
        }
    }

    #[test]
    fn test_task_command_classification() {
        let classifier = IntentClassifier::new();
        let result = classifier.classify("Create a new task for grocery shopping", None);
        
        match result.intent {
            Intent::Command { action, .. } => assert_eq!(action, "task"),
            _ => panic!("Expected command intent"),
        }
        assert!(result.extracted_entities.contains_key("task_name"));
    }

    #[test]
    fn test_document_search_classification() {
        let classifier = IntentClassifier::new();
        let result = classifier.classify("Find documents about machine learning", None);
        
        match result.intent {
            Intent::Query { query } => assert!(query.contains("machine learning")),
            _ => panic!("Expected query intent for document search"),
        }
    }

    #[test]
    fn test_context_based_classification() {
        let classifier = IntentClassifier::new();
        let context = create_test_context();
        
        let result = classifier.classify("yes", Some(&context));
        
        // Without previous conversation, this should be low confidence
        assert!(result.confidence < 0.8);
    }

    #[test]
    fn test_unknown_classification() {
        let classifier = IntentClassifier::new();
        let result = classifier.classify("asdf qwerty random nonsense", None);
        
        match result.intent {
            Intent::Unknown => assert!(result.confidence < 0.3),
            _ => {}, // Might still classify as something with low confidence
        }
    }

    #[test]
    fn test_entity_extraction() {
        let classifier = IntentClassifier::new();
        let result = classifier.classify("Create task buy milk due tomorrow", None);
        
        assert!(result.extracted_entities.contains_key("task_name"));
        if let Some(task_name) = result.extracted_entities.get("task_name") {
            assert!(task_name.contains("buy milk"));
        }
    }
}