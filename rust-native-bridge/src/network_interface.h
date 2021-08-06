#include <stdint.h>

typedef struct SwapiClient SwapiClient;

typedef struct {
  void *owner;
  void (*onResult)(void *owner, const char *arg);
  void (*onError)(void *owner, const char *arg);
} NetReqCallback;

SwapiClient *create_swapi_client(void);
void free_swapi_client(SwapiClient *client);
void http_request_post(SwapiClient *client, const char *url, const char *parm_str, NetReqCallback callback);
