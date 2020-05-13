#include <cstdlib>
#include <cstring>
#include <type_traits>
#include <vector>

#include <sentencepiece_processor.h>

using absl::string_view;
using sentencepiece::SentencePieceProcessor;
using sentencepiece::SentencePieceText;

// Inspired by:
// https://stackoverflow.com/a/14589519
template<typename E>
constexpr auto to_underlying_type(E e) -> typename std::underlying_type<E>::type 
{
   return static_cast<typename std::underlying_type<E>::type>(e);
}

extern "C" {

SentencePieceProcessor *spp_new() {
  return new SentencePieceProcessor();
}

unsigned char *spp_encode_as_serialized_proto(SentencePieceProcessor *spp, char const *sentence, size_t *len) {
  auto serialized = spp->EncodeAsSerializedProto(sentence);

  *len = serialized.size();
  unsigned char *data = (unsigned char *) malloc(serialized.size());
  memcpy(data, serialized.data(), serialized.size());

  return data;
}

int spp_load(SentencePieceProcessor *spp, char const *filename) {
  auto status = spp->Load(filename);
  return to_underlying_type(status.code());
}

bool spp_is_unknown(SentencePieceProcessor *spp, int id) {
  return spp->IsUnknown(id);
}

int spp_piece_to_id(SentencePieceProcessor *spp, char const *piece) {
  return spp->PieceToId(piece);
}

int spp_from_serialized_proto(SentencePieceProcessor *spp, char const *data, size_t len) {
  auto status = spp->LoadFromSerializedProto(string_view(data, len));
  return to_underlying_type(status.code());
}

void spp_free(SentencePieceProcessor *spp) {
  delete spp;
}

int spp_unknown_id(SentencePieceProcessor *spp) {
  return spp->unk_id();
}

}
