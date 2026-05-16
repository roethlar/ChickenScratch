import { useMemo, useState } from "react";
import { useProjectStore } from "../../stores/projectStore";
import type { Document, Thread } from "../../types";

interface TimelineScene {
  doc: Document;
  /** Seconds since epoch, or NaN if unparseable */
  time: number;
  /** minutes of story time; 0 if not set */
  duration: number;
  /** Normalized for display */
  displayTime: string;
  pov: string | null;
  threads: string[];
}

interface TimelineData {
  scenes: TimelineScene[];
  invalidStoryTimes: number;
}

function parseStoryTime(raw: unknown): { time: number; display: string } {
  const str = String(raw ?? "").trim();
  if (!str) return { time: NaN, display: "" };

  // ISO 8601: 2024-03-15T22:30 or 2024-03-15
  const iso = /^\d{4}-\d{2}-\d{2}(?:T\d{2}:\d{2}(?::\d{2})?)?/.exec(str);
  if (iso) {
    const d = new Date(iso[0]);
    if (!isNaN(d.getTime())) {
      return { time: d.getTime() / 1000, display: str };
    }
  }

  // Try to extract a leading number for ordering (e.g. "Day 3, 22:30" -> 3)
  const num = /(\d+)/.exec(str);
  if (num) {
    return { time: parseInt(num[1], 10), display: str };
  }

  // Fallback: alphabetical, show as-is
  return { time: NaN, display: str };
}

function hasStoryTime(raw: unknown): boolean {
  return String(raw ?? "").trim().length > 0;
}

function extractTimelineData(project: NonNullable<ReturnType<typeof useProjectStore.getState>["project"]>): TimelineData {
  const scenes: TimelineScene[] = [];
  let invalidStoryTimes = 0;
  for (const doc of Object.values(project.documents)) {
    const fields = doc.fields ?? {};
    if (!hasStoryTime(fields.story_time)) continue;

    const st = parseStoryTime(fields.story_time);
    if (!isNaN(st.time)) {
      const dur = typeof fields.duration_minutes === "number" ? fields.duration_minutes : 0;
      const pov = typeof fields.pov_character === "string" ? fields.pov_character : null;
      const threads = Array.isArray(fields.threads)
        ? fields.threads.filter((t): t is string => typeof t === "string")
        : [];
      scenes.push({ doc, time: st.time, duration: dur, displayTime: st.display, pov, threads });
    } else {
      invalidStoryTimes += 1;
    }
  }
  return { scenes: scenes.sort((a, b) => a.time - b.time), invalidStoryTimes };
}

type LaneMode = "pov" | "thread" | "single";

export function TimelineView() {
  const project = useProjectStore((s) => s.project);
  const selectDocument = useProjectStore((s) => s.selectDocument);
  const [laneMode, setLaneMode] = useState<LaneMode>("pov");

  const timelineData = useMemo(() => (project ? extractTimelineData(project) : { scenes: [], invalidStoryTimes: 0 }), [project]);
  const { scenes, invalidStoryTimes } = timelineData;

  const allThreads: Thread[] = project?.threads ?? [];

  const { lanes, unplaced } = useMemo(() => {
    const placed: Map<string, TimelineScene[]> = new Map();
    const unplacedScenes: TimelineScene[] = [];

    for (const s of scenes) {
      if (laneMode === "single") {
        const key = "Chronological";
        const arr = placed.get(key) ?? [];
        arr.push(s);
        placed.set(key, arr);
      } else if (laneMode === "pov") {
        const key = s.pov || "Unknown POV";
        const arr = placed.get(key) ?? [];
        arr.push(s);
        placed.set(key, arr);
      } else {
        // thread mode: scene appears in every thread lane it belongs to
        if (s.threads.length === 0) {
          unplacedScenes.push(s);
        } else {
          for (const t of s.threads) {
            const arr = placed.get(t) ?? [];
            arr.push(s);
            placed.set(t, arr);
          }
        }
      }
    }

    if (laneMode === "single" || laneMode === "pov") {
      // No unplaced in these modes
    }

    const sortedLanes = Array.from(placed.entries()).sort((a, b) => a[0].localeCompare(b[0]));
    return { lanes: sortedLanes, unplaced: unplacedScenes };
  }, [scenes, laneMode]);

  // Compute time range for scaling
  const timeRange = useMemo(() => {
    const times = scenes.map((s) => s.time).filter((t) => !isNaN(t));
    if (times.length === 0) return { min: 0, max: 1 };
    return { min: Math.min(...times), max: Math.max(...times) };
  }, [scenes]);

  const scale = (t: number) => {
    if (timeRange.max === timeRange.min) return 50;
    return ((t - timeRange.min) / (timeRange.max - timeRange.min)) * 100;
  };

  const durationWidth = (duration: number) => {
    if (duration <= 0 || timeRange.max === timeRange.min) return undefined;
    return `${Math.max(2, (duration / Math.max(1, timeRange.max - timeRange.min)) * 100)}%`;
  };

  const threadColor = (id: string) => {
    const t = allThreads.find((x) => x.id === id || x.name === id);
    return t?.color || "#888";
  };

  const handleSceneClick = (docId: string) => {
    selectDocument(docId);
  };

  if (!project) {
    return (
      <div className="timeline-empty">
        <p>Open a project to view the timeline.</p>
      </div>
    );
  }

  if (scenes.length === 0) {
    if (invalidStoryTimes > 0) {
      return (
        <div className="timeline-empty">
          <p>No valid Story Time values yet.</p>
        </div>
      );
    }

    return (
      <div className="timeline-empty">
        <p>No scenes with a Story Time yet.</p>
        <p className="timeline-hint">
          Add a Story Time to scenes in the Inspector to see them here.
        </p>
      </div>
    );
  }

  return (
    <div className="timeline">
      <div className="timeline-toolbar">
        <span className="timeline-title">Timeline</span>
        <div className="timeline-lane-toggle">
          {(["pov", "thread", "single"] as LaneMode[]).map((m) => (
            <button
              key={m}
              className={laneMode === m ? "active" : ""}
              onClick={() => setLaneMode(m)}
            >
              {m === "pov" ? "POV" : m === "thread" ? "Thread" : "Single"}
            </button>
          ))}
        </div>
      </div>

      <div className="timeline-scroll">
        <div className="timeline-ruler">
          <div className="timeline-ruler-line" />
        </div>

        {lanes.map(([laneName, laneScenes]) => (
          <div key={laneName} className="timeline-lane">
            <div className="timeline-lane-label">{laneName}</div>
            <div className="timeline-lane-track">
              {laneScenes.map((s) => (
                <div
                  key={s.doc.id}
                  className="timeline-chip"
                  style={{
                    left: `${scale(s.time)}%`,
                    width: durationWidth(s.duration),
                  }}
                  onClick={() => handleSceneClick(s.doc.id)}
                  title={`${s.doc.name}\n${s.displayTime}\n${s.doc.synopsis || ""}`}
                >
                  <div className="timeline-chip-name">{s.doc.name}</div>
                  <div className="timeline-chip-time">{s.displayTime}</div>
                  {s.doc.synopsis && (
                    <div className="timeline-chip-synopsis">{s.doc.synopsis}</div>
                  )}
                  {s.threads.length > 0 && (
                    <div className="timeline-chip-threads">
                      {s.threads.map((t) => (
                        <span
                          key={t}
                          className="timeline-thread-dot"
                          style={{ backgroundColor: threadColor(t) }}
                          title={t}
                        />
                      ))}
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>
        ))}

        {unplaced.length > 0 && (
          <div className="timeline-lane unplaced">
            <div className="timeline-lane-label">Unplaced</div>
            <div className="timeline-lane-track">
              {unplaced.map((s) => (
                <div
                  key={s.doc.id}
                  className="timeline-chip"
                  onClick={() => handleSceneClick(s.doc.id)}
                  title={s.doc.name}
                >
                  <div className="timeline-chip-name">{s.doc.name}</div>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
