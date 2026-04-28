/*
 * C ABI smoke test for adventerm_ffi (M9).
 *
 * Exercises a thin slice of the FFI surface end-to-end:
 *   game alloc -> player_pos -> action_move -> inventory_len ->
 *   save round-trip -> restore -> assert position preserved -> free.
 *
 * Cargo only picks up `*.rs` files for integration tests, so this file
 * lives under `tests/` without being compiled by `cargo test`. The CI
 * step (see `plans/ffi/07-build-and-test.md`) builds and runs it via
 * `cc` directly against `target/release/libadventerm_ffi`.
 */
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "adventerm_ffi.h"

int main(void) {
    GameHandle* game = game_new_seeded(42);
    assert(game != NULL);

    size_t x = 0, y = 0;
    int rc = game_player_pos(game, &x, &y);
    assert(rc == 0);
    printf("player at %zu,%zu\n", x, y);

    /* CDirection::Up = 0 (see adventerm_ffi/src/enums.rs). The enum is not
     * referenced from any extern "C" signature, so cbindgen does not emit
     * its declaration; pass the raw discriminant. */
    CMoveOutcome outcome = {0};
    rc = game_action_move(game, 0, &outcome);
    assert(rc == 0);
    printf("move outcome tag=%u\n", outcome.tag);

    size_t inv_len = 0;
    rc = game_inventory_len(game, &inv_len);
    assert(rc == 0);

    SaveHandle* save = NULL;
    rc = save_new_from_game(game, "smoke", &save);
    assert(rc == 0);
    assert(save != NULL);

    /* Two-call buffer-discovery pattern: first call returns BufferTooSmall
     * (-12) and writes the required byte count to `out_required`. */
    size_t needed = 0;
    rc = save_to_bytes(save, NULL, 0, &needed);
    assert(rc == -12);
    assert(needed > 0);

    uint8_t* buf = malloc(needed);
    assert(buf != NULL);
    rc = save_to_bytes(save, buf, needed, &needed);
    assert(rc == 0);

    SaveHandle* restored = NULL;
    rc = save_from_bytes(buf, needed, &restored);
    assert(rc == 0);
    assert(restored != NULL);

    GameHandle* restored_game = NULL;
    rc = save_to_game(restored, &restored_game);
    assert(rc == 0);
    assert(restored_game != NULL);

    size_t rx = 0, ry = 0;
    rc = game_player_pos(restored_game, &rx, &ry);
    assert(rc == 0);
    assert(rx == x && ry == y);

    free(buf);
    save_free(save);
    save_free(restored);
    game_free(restored_game);
    game_free(game);

    printf("smoke OK\n");
    return 0;
}
