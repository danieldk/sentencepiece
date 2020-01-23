#include <cstdlib>
#include <cstring>
#include <vector>

#include <sentencepiece_processor.h>

using sentencepiece::SentencePieceProcessor;
using sentencepiece::SentencePieceText;

extern "C" {

SentencePieceProcessor *spp_new() {
  return new SentencePieceProcessor();
}

unsigned char *spp_encode_as_serialized_proto(SentencePieceProcessor *spp, char const *sentence, size_t *len) {
  auto serialized = spp->EncodeAsSerializedProto(sentence);

  *len = serialized.size();
  unsigned char *data = (unsigned char * ) malloc(serialized.size());
  memcpy(data, serialized.data(), serialized.size());

  return data;
}

int spp_load(SentencePieceProcessor *spp, char const *filename) {
  auto status = spp->Load(filename);
  return status.code();
}

void spp_free(SentencePieceProcessor *spp) {
  delete spp;
}

}
