#pragma once
#ifdef __cplusplus
extern "C" {
#endif

char* zone_publish(const char* node_url, const char* signing_key_hex, const char* data);
void zone_free_string(char* s);

#ifdef __cplusplus
}
#endif
