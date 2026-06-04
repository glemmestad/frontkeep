# Robotics & control domain overlay

Pulled in when the work drives physical or simulated robots: control loops,
sensors, actuators, ROS, SLAM, kinematics, motion planning, or teleoperation.
The difference from ordinary software is that a bug moves real mass — it can
break hardware or hurt a person, so safety is a hard requirement, not a feature.

## Safety is non-negotiable
- Define and test the safe state and how the system reaches it on any fault: e-stop, watchdog timeout, lost comms, sensor dropout. A control loop that doesn't fail safe is not done.
- Bound every actuator command (position, velocity, torque, current) before it leaves software. Clamp at the boundary; never trust an upstream value to already be in range.
- Test new control logic in simulation and against limits before it touches hardware. The first real-hardware run is supervised, with an e-stop in reach.

## Sensors & state estimation
- Every sensor reading can be late, missing, noisy, or wrong. Timestamp inputs, reject stale or out-of-range data, and don't act on a single unfiltered reading.
- Be explicit about coordinate frames, units, and conventions, and the transforms between them. A frame or unit mismatch is the classic robotics bug — it looks fine until the arm swings the wrong way.

## Real-time control
- Control loops have a deadline. State the loop rate and ensure the worst-case path holds it; a loop that occasionally misses its period is an instability, not a hiccup.
- Keep the control path deterministic: no blocking I/O, logging, or allocation inside the loop. Push that work off the hot path.
- Account for latency and jitter end to end (sense → compute → actuate). Tune and validate against the real delay, not zero.

## Done bar
A robotics change is done when the safe state and every fault path (e-stop,
watchdog, lost comms, bad sensor) are defined and tested, actuator commands are
bounded at the boundary, frames/units are explicit and checked, the control loop
meets its deadline on the worst-case path with no blocking work inside it, and the
change was validated in simulation before any supervised hardware run.
