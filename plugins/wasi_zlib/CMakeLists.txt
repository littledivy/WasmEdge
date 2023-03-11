include(FetchContent)

FetchContent_Declare(
  Corrosion
  GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
  GIT_TAG v0.3.2
)

FetchContent_MakeAvailable(Corrosion)

corrosion_import_crate(MANIFEST_PATH Cargo.toml)

corrosion_set_env_vars(wasi_zlib WASMEDGE_INCLUDE_DIR=${CMAKE_SOURCE_DIR}/build/include/api)

corrosion_link_libraries(wasi_zlib
  wasmedge_shared
)

if(NOT WASMEDGE_LINK_PLUGINS_STATIC)
  set(BUILD_SHARED_LIBS ON)
else()

endif()

install(TARGETS wasi_zlib DESTINATION ${CMAKE_INSTALL_LIBDIR}/wasmedge)
