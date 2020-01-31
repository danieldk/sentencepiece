#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct SentencePieceProcessor SentencePieceProcessor;

typedef struct SentencePieceText SentencePieceText;

unsigned char *spp_encode_as_serialized_proto(SentencePieceProcessor *spp, char const *sentence, size_t *len);

SentencePieceProcessor *spp_new();

int spp_from_serialized_proto(SentencePieceProcessor *spp, char const *data, size_t len);

int spp_load(SentencePieceProcessor *spp, char const *filename);

void spp_free(SentencePieceProcessor *spp);

#ifdef __cplusplus
}
#endif
