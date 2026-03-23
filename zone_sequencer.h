#pragma once
#ifdef __cplusplus
extern "C" {
#endif
// Publish data. checkpoint_path: file to load/save checkpoint ("" to disable).
// Returns hex inscription ID (caller must free with zone_free_string), or NULL on error.
char* zone_publish(const char* node_url, const char* signing_key_hex, const char* data, const char* checkpoint_path);

// Query inscriptions from a zone channel.
// Returns JSON array string: [{"id":"hex","data":"text"}, ...] or NULL on error.
// Caller must free with zone_free_string().
char* zone_query_channel(const char* node_url, const char* channel_id_hex, int limit);

void zone_free_string(char* s);
#ifdef __cplusplus
}
#endif
