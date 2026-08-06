import tempfile
import unittest
from pathlib import Path

from event_buffer import EventBuffer


class TestEventBuffer(unittest.IsolatedAsyncioTestCase):

    async def test_event_buffer_flow(self):

        with tempfile.TemporaryDirectory() as temp_dir:

            db_path = Path(temp_dir) / "events.db"

            # Create buffer with max_events = 3
            buffer = EventBuffer(db_path, 3)

            # Check empty
            self.assertTrue(await buffer.is_empty())
            self.assertEqual(await buffer.len(), 0)

            # Push 4 events (oldest should be evicted)
            await buffer.push("event 1")
            await buffer.push("event 2")
            await buffer.push("event 3")
            await buffer.push("event 4")

            # Capacity should be 3
            self.assertEqual(await buffer.len(), 3)

            # Drain 2 events
            drained = await buffer.drain(2)

            self.assertEqual(len(drained), 2)
            self.assertEqual(drained[0], "event 2")
            self.assertEqual(drained[1], "event 3")

            # One event should remain
            self.assertEqual(await buffer.len(), 1)

            remaining = await buffer.drain(5)

            self.assertEqual(len(remaining), 1)
            self.assertEqual(remaining[0], "event 4")

            # Buffer should now be empty
            self.assertTrue(await buffer.is_empty())
