/*
 * JJS main API
 *
 * No description provided (generated by Openapi Generator https://github.com/openapitools/openapi-generator)
 *
 * The version of the OpenAPI document: 1.0.0
 *
 * Generated by: https://openapi-generator.tech
 */

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Participation {
    #[serde(rename = "phase")]
    pub phase: String,
}

impl Participation {
    pub fn new(phase: String) -> Participation {
        Participation { phase }
    }
}