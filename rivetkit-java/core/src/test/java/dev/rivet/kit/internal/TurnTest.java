package dev.rivet.kit.internal;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import org.junit.jupiter.api.Test;

class TurnTest {
    @Test
    void ticketsRunInReservationOrderOneAtATime() throws Exception {
        Turn turn = new Turn();
        List<Integer> order = Collections.synchronizedList(new ArrayList<>());
        AtomicInteger inside = new AtomicInteger();
        AtomicInteger maxInside = new AtomicInteger();
        int n = 200;
        CountDownLatch done = new CountDownLatch(n);
        List<Turn.Ticket> tickets = new ArrayList<>();
        for (int i = 0; i < n; i++) tickets.add(turn.reserve());
        // Start threads in reverse: order must still follow reservation.
        for (int i = n - 1; i >= 0; i--) {
            int id = i;
            Turn.Ticket ticket = tickets.get(i);
            Thread.ofVirtual().start(() -> {
                turn.await(ticket);
                maxInside.accumulateAndGet(inside.incrementAndGet(), Math::max);
                order.add(id);
                inside.decrementAndGet();
                turn.release(ticket);
                done.countDown();
            });
        }
        assertTrue(done.await(10, TimeUnit.SECONDS), "every holder finished");
        assertEquals(1, maxInside.get(), "never two holders at once");
        for (int i = 0; i < n; i++) assertEquals(i, order.get(i));
    }

    @Test
    void onlyTheOwnerReleases() {
        Turn turn = new Turn();
        Turn.Ticket first = turn.reserve();
        Turn.Ticket second = turn.reserve();
        assertThrows(IllegalStateException.class, () -> turn.release(second));
        turn.release(first);
        assertFalse(turn.owns(first));
        assertEquals(0, turn.waiting());
    }
}
