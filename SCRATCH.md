# SCRATCH

Temporary notes and ideas.

## JSON mapping to tasks

```json
{
  "project": {
    "id": 1,
    "name": "Build your own HTTP Server",
    "runner_image": "local|go|rust|c",
    "tasks": [
      {
        "id": 1,
        "name": "Setup basic HTTP server",
        "description": "Create a simple HTTP server that responds to GET requests.",
        "scores": "10:12:15|15:20:7",
        "status": "challenge_awaits",
        "abandoned_deduction": 3,
        "dependencies": [],
        "hints": [
          {
            "id": 1,
            "text": "Remember to handle multiple connections concurrently.",
            "unlock_criteria": "10:30:T",
            "points_deduction": 5
          }
        ],
        "validators": [
          "file_exists:main.go",
          "can_compile",
          "running_on_port:8000"
        ]
      }
    ]
  }
}
```

## Field Documentation

### runner_image
Format: `"local|go|rust|c"`
- `local` = no runtime needed
- Uses `|` delimiter (not comma) to reserve commas for future tuple syntax (e.g., multi-service: web server + database)

### scores
Format: `"attempts:minutes:points|..."`
- Example: `"10:12:15|15:20:7"` means:
  - Bucket 1: 10 attempts OR 12 minutes → 15 points
  - Bucket 2: 15 attempts OR 20 minutes → 7 points
- Bucket selection: whichever threshold is hit first (`min(attempts, time)`)

### hints.unlock_criteria
Format: `"time:attempts:priority"`
- Example: `"10:30:T"` means unlock after 10 minutes OR 30 attempts, prioritize time (T)
- Priority: `T` = time-based, `A` = attempt-based

### status
Enum: `challenge_awaits | challenged | challenge_completed | challenge_failed | challenge_abandoned`

| Status | Description |
|--------|-------------|
| `challenge_awaits` | User has not started this task yet |
| `challenged` | User has started and is actively working |
| `challenge_completed` | User successfully finished the task |
| `challenge_failed` | User explicitly gave up or hit max attempts |
| `challenge_abandoned` | No activity for N time (set by backend job) |

State transitions:
```
challenge_awaits → challenged              (user starts task)
challenged → challenge_completed           (user passes all validators)
challenged → challenge_failed              (user gives up or max attempts)
challenged → challenge_abandoned           (no activity, backend job checks updated_at)
challenge_abandoned → challenged           (user returns and resumes)
```

### abandoned_deduction
Points deducted when user resumes an abandoned task.

Backend job runs every ~10 mins, checks `updated_at` column to determine if task should be marked `abandoned`.

### validators
- Beginner projects: 1 validator per task
- Advanced projects: multiple validators per task (harder to earn points)
- Hints and points are based on "Task" level, not individual validations
