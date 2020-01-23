use cloudevent::Event;
use std::collections::HashMap;

pub enum EventRequest {
    Binary(Option<Event>),
    Structured(Option<Event>),
    Batch(Vec<Event>),
    Bundle(HashMap<String, Event>)
}

pub enum EventResponse {
    Binary(Option<Event>),
    Structured(Option<Event>),
    Batch(Vec<Event>),
    Bundle(HashMap<String, Event>)
}
