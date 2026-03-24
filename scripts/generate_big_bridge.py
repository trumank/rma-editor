#!/usr/bin/env python3
"""
Generate a BigBridge room JSON.

BigBridge rooms feature:
- A large FloodFillLine forming the main cave
- Multiple FloodFillPillars forming bridges through the cave
- Entrances placed at cave endpoints
"""

import argparse
import json
import math
import random
import sys
from dataclasses import dataclass


@dataclass
class Vec3:
    x: float
    y: float
    z: float

    def __add__(self, other: "Vec3") -> "Vec3":
        return Vec3(self.x + other.x, self.y + other.y, self.z + other.z)

    def __sub__(self, other: "Vec3") -> "Vec3":
        return Vec3(self.x - other.x, self.y - other.y, self.z - other.z)

    def __mul__(self, scalar: float) -> "Vec3":
        return Vec3(self.x * scalar, self.y * scalar, self.z * scalar)

    def __rmul__(self, scalar: float) -> "Vec3":
        return self * scalar

    def dot(self, other: "Vec3") -> float:
        return self.x * other.x + self.y * other.y + self.z * other.z

    def length(self) -> float:
        return math.sqrt(self.dot(self))

    def normalized(self) -> "Vec3":
        l = self.length()
        if l < 1e-10:
            return Vec3(0, 0, 0)
        return Vec3(self.x / l, self.y / l, self.z / l)

    def to_list(self) -> list:
        return [round(self.x, 4), round(self.y, 4), round(self.z, 4)]

    @staticmethod
    def from_list(lst: list) -> "Vec3":
        return Vec3(lst[0], lst[1], lst[2])


# === FloodFillLine (main cave) parameters ===

# Number of points in the main cave line
CAVE_NUM_POINTS = 12 # 12

# Step distance between cave points
CAVE_STEP_MIN = 2000.0
CAVE_STEP_MAX = 4000.0

# Horizontal range (cave width)
CAVE_HRANGE_MIN = 1400.0
CAVE_HRANGE_MAX = 3500.0

# Vertical range (cave depth/height)
CAVE_VRANGE_MIN = 3000.0
CAVE_VRANGE_MAX = 6000.0

# Noise ranges
CAVE_CEILING_NOISE = 100.0
CAVE_WALL_NOISE = 100.0
CAVE_FLOOR_NOISE = 100.0
CAVE_CEILING_HEIGHT = 10558.616
CAVE_HEIGHT_SCALE = 1.0
CAVE_FLOOR_DEPTH = 0.0

# Floor angle range (degrees)
CAVE_FLOOR_ANGLE_MIN = -25.0
CAVE_FLOOR_ANGLE_MAX = 25.0

# Vertical movement per step
CAVE_VERTICAL_STEP_MIN = -1500.0
CAVE_VERTICAL_STEP_MAX = 1500.0

# Horizontal turn rate (radians)
CAVE_TURN_RATE = 0.8

# Starting position
CAVE_START_X = 0.0
CAVE_START_Y = 0.0
CAVE_START_Z = 0.0


# === FloodFillPillar (bridge) parameters ===

# Number of bridges to generate
NUM_BRIDGES = 10

# Bridge radius range
BRIDGE_RANGE_MIN = 250.0
BRIDGE_RANGE_MAX = 400.0

# Bridge noise range
BRIDGE_NOISE_MIN = 80.0
BRIDGE_NOISE_MAX = 200.0

# Target segment length between waypoints
BRIDGE_SEGMENT_LENGTH_MIN = 1500.0
BRIDGE_SEGMENT_LENGTH_MAX = 2000.0

# Minimum number of waypoints for a valid bridge (excluding endpoints)
BRIDGE_MIN_WAYPOINTS = 2

# Max consecutive failures before terminating bridge
BRIDGE_MAX_CONSECUTIVE_FAILURES = 10

# Collision margin between bridges (in addition to their radii)
BRIDGE_COLLISION_MARGIN = 300.0

# Minimum SDF value for endpoints (positive = in rock)
BRIDGE_ENDPOINT_MIN_SDF = 200.0

# Maximum SDF value for waypoints (negative = inside cave, with margin from walls)
BRIDGE_WAYPOINT_MAX_SDF = -800.0

# Maximum attempts to place a bridge before giving up
BRIDGE_MAX_ATTEMPTS = 50


@dataclass
class CaveSegment:
    """A segment of the cave between two FloodFillLine points."""
    start: Vec3
    end: Vec3
    start_hrange: float
    start_vrange: float
    end_hrange: float
    end_vrange: float
    start_floor_depth: float
    start_floor_angle: float
    end_floor_depth: float
    end_floor_angle: float

    def interpolate(self, t: float) -> tuple[Vec3, float, float]:
        """Get position and cross-section size at parameter t (0-1)."""
        pos = self.start + (self.end - self.start) * t
        hrange = self.start_hrange + (self.end_hrange - self.start_hrange) * t
        vrange = self.start_vrange + (self.end_vrange - self.start_vrange) * t
        return pos, hrange, vrange

    def interpolate_floor(self, t: float) -> tuple[float, float]:
        """Get floor depth and angle at parameter t (0-1)."""
        floor_depth = self.start_floor_depth + (self.end_floor_depth - self.start_floor_depth) * t
        floor_angle = self.start_floor_angle + (self.end_floor_angle - self.start_floor_angle) * t
        return floor_depth, floor_angle


class CaveVolume:
    """Represents the cave volume for point-in-cave testing and sampling."""

    def __init__(self, flood_fill_line: dict):
        points = flood_fill_line["Points"]
        self.segments: list[CaveSegment] = []
        self.total_length = 0.0
        self.segment_lengths: list[float] = []
        self.points = points  # Store raw points for SDF

        for i in range(len(points) - 1):
            p0 = Vec3.from_list(points[i]["Location"])
            p1 = Vec3.from_list(points[i + 1]["Location"])
            seg = CaveSegment(
                start=p0,
                end=p1,
                start_hrange=points[i]["HRange"],
                start_vrange=points[i]["VRange"],
                end_hrange=points[i + 1]["HRange"],
                end_vrange=points[i + 1]["VRange"],
                start_floor_depth=points[i].get("FloorDepth", 0.0),
                start_floor_angle=points[i].get("FloorAngle", 0.0),
                end_floor_depth=points[i + 1].get("FloorDepth", 0.0),
                end_floor_angle=points[i + 1].get("FloorAngle", 0.0),
            )
            self.segments.append(seg)
            seg_len = (p1 - p0).length()
            self.segment_lengths.append(seg_len)
            self.total_length += seg_len

    def sample_along_centerline(self, t: float) -> tuple[Vec3, float, float, Vec3]:
        """
        Sample a position along the cave centerline.
        t: 0-1 parameter along entire cave length
        Returns: (position, hrange, vrange, direction)
        """
        target_dist = t * self.total_length
        accum_dist = 0.0

        for i, (seg, seg_len) in enumerate(zip(self.segments, self.segment_lengths)):
            if accum_dist + seg_len >= target_dist or i == len(self.segments) - 1:
                local_t = (target_dist - accum_dist) / seg_len if seg_len > 0 else 0
                local_t = max(0, min(1, local_t))
                pos, hrange, vrange = seg.interpolate(local_t)
                direction = (seg.end - seg.start).normalized()
                return pos, hrange, vrange, direction
            accum_dist += seg_len

        # Fallback (shouldn't reach here)
        seg = self.segments[-1]
        pos, hrange, vrange = seg.interpolate(1.0)
        direction = (seg.end - seg.start).normalized()
        return pos, hrange, vrange, direction

    def segment_sdf(self, seg: CaveSegment, p: Vec3) -> float:
        """
        SDF for a single cave segment (ellipsoid tunnel with floor).
        Negative = inside cave, Positive = outside (in rock)
        """
        a = seg.start
        b = seg.end
        ba = b - a
        pa = p - a
        ba_len_sq = ba.dot(ba)

        # Project point onto segment
        if ba_len_sq > 1e-12:
            t = pa.dot(ba) / ba_len_sq
        else:
            t = 0.5
        t = max(0.0, min(1.0, t))

        # Interpolate radii
        r_h = max(0.01, seg.start_hrange * (1.0 - t) + seg.end_hrange * t)
        r_v = max(0.01, seg.start_vrange * (1.0 - t) + seg.end_vrange * t)

        # Interpolate floor parameters
        floor_depth, floor_angle = seg.interpolate_floor(t)

        # Point on segment axis
        segment_point = a + ba * t
        offset = p - segment_point

        # Calculate perpendicular direction for floor angle
        horizontal_dir_len = math.sqrt(ba.x * ba.x + ba.y * ba.y)
        if horizontal_dir_len > 1e-6:
            right_x = -ba.y / horizontal_dir_len
            right_y = ba.x / horizontal_dir_len
        else:
            right_x, right_y = 1.0, 0.0
        perp_dist = offset.x * right_x + offset.y * right_y

        # Ellipsoid SDF with radii (r_h, r_h, r_v)
        qx = offset.x / r_h
        qy = offset.y / r_h
        qz = offset.z / r_v
        k0 = math.sqrt(qx * qx + qy * qy + qz * qz)

        q2x = offset.x / (r_h * r_h)
        q2y = offset.y / (r_h * r_h)
        q2z = offset.z / (r_v * r_v)
        k1 = math.sqrt(q2x * q2x + q2y * q2y + q2z * q2z)

        if k1 > 1e-10:
            ellipsoid_dist = k0 * (k0 - 1.0) / k1
        else:
            ellipsoid_dist = float('inf')

        # Floor constraint with angle (tilted plane)
        angle_rad = math.radians(floor_angle)
        angle_offset = -math.sin(angle_rad) * perp_dist
        floor_z = segment_point.z + floor_depth + angle_offset
        floor_dist = floor_z - p.z  # Positive when point is below floor

        # Combine: inside cave = inside ellipsoid AND above floor
        return max(ellipsoid_dist, floor_dist)

    def sdf(self, p: Vec3) -> float:
        """
        Signed distance function for the entire cave.
        Negative = inside cave, Positive = outside (in rock)
        """
        min_dist = float('inf')
        for seg in self.segments:
            d = self.segment_sdf(seg, p)
            min_dist = min(min_dist, d)
        return min_dist

    def is_inside(self, point: Vec3, margin: float = 0.0) -> bool:
        """Check if a point is inside the cave volume (with optional margin)."""
        return self.sdf(point) < -margin


def segment_segment_distance(p0: Vec3, p1: Vec3, q0: Vec3, q1: Vec3) -> float:
    """Compute minimum distance between two line segments."""
    d1 = p1 - p0  # Direction of segment 1
    d2 = q1 - q0  # Direction of segment 2
    r = p0 - q0

    a = d1.dot(d1)
    b = d1.dot(d2)
    c = d2.dot(d2)
    d = d1.dot(r)
    e = d2.dot(r)

    denom = a * c - b * b

    # Parameters for closest points
    if denom < 1e-10:
        # Parallel segments
        s = 0.0
        t = d / b if abs(b) > 1e-10 else 0.0
    else:
        s = (b * e - c * d) / denom
        t = (a * e - b * d) / denom

    # Clamp to segment bounds
    s = max(0, min(1, s))
    t = max(0, min(1, t))

    # Recompute to handle clamping
    if s < 0 or s > 1:
        s = max(0, min(1, s))
        t = (b * s + e) / c if abs(c) > 1e-10 else 0
        t = max(0, min(1, t))
    if t < 0 or t > 1:
        t = max(0, min(1, t))
        s = (-b * t - d) / a if abs(a) > 1e-10 else 0
        s = max(0, min(1, s))

    closest_p = p0 + d1 * s
    closest_q = q0 + d2 * t

    return (closest_p - closest_q).length()


def bridges_collide(bridge1: list[Vec3], radius1: float,
                    bridge2: list[Vec3], radius2: float,
                    margin: float) -> bool:
    """Check if two bridges (as point lists) collide."""
    min_dist = radius1 + radius2 + margin

    for i in range(len(bridge1) - 1):
        for j in range(len(bridge2) - 1):
            dist = segment_segment_distance(
                bridge1[i], bridge1[i + 1],
                bridge2[j], bridge2[j + 1]
            )
            if dist < min_dist:
                return True
    return False


def calculate_path_length(points: list[Vec3]) -> float:
    """Calculate total length of a path through points."""
    total = 0.0
    for i in range(len(points) - 1):
        total += (points[i + 1] - points[i]).length()
    return total


def find_wall_point(cave: CaveVolume, center: Vec3, direction: Vec3,
                    target_sdf: float, max_dist: float = 5000.0) -> Vec3 | None:
    """
    March from center along direction until we reach target SDF value.
    Returns the point, or None if we exceed max_dist.
    """
    step = 100.0
    dist = 0.0

    while dist < max_dist:
        p = center + direction * dist
        sdf = cave.sdf(p)
        if sdf >= target_sdf:
            return p
        dist += step

    return None


def bias_toward_ends(rng: random.Random) -> float:
    """
    Generate a t value (0-1) biased toward the ends of the cave.
    Uses a U-shaped distribution.
    """
    # Generate value biased toward 0 or 1
    t = rng.random()
    # Transform to bias toward ends: use squared distance from center
    if t < 0.5:
        # Map [0, 0.5] to [0, 0.5] with bias toward 0
        return 0.5 * (1 - math.sqrt(1 - 2 * t))
    else:
        # Map [0.5, 1] to [0.5, 1] with bias toward 1
        return 0.5 + 0.5 * math.sqrt(2 * t - 1)


def generate_bridge(rng: random.Random, cave: CaveVolume,
                    existing_bridges: list[tuple[list[Vec3], float]]) -> dict | None:
    """
    Attempt to generate a bridge that doesn't collide with existing ones.
    Bridge starts from a wall (biased toward cave ends), then iteratively extends
    until it can no longer place waypoints inside the cave, then terminates in wall.
    Returns None if unable to place after max attempts.
    """
    for _ in range(BRIDGE_MAX_ATTEMPTS):
        # Pick starting position biased toward ends of cave
        t_current = bias_toward_ends(rng)
        t_current = max(0.05, min(0.95, t_current))

        # Get cave info at start
        pos_start, hrange_start, vrange_start, dir_start = cave.sample_along_centerline(t_current)

        # Perpendicular direction
        perp = Vec3(-dir_start.y, dir_start.x, 0).normalized()

        # Pick which wall to start from
        wall_side = rng.choice([-1, 1])

        # Pick Z height (upper portion of cave)
        z_ratio = rng.uniform(0.2, 0.7)
        bridge_z = pos_start.z + vrange_start * z_ratio

        # Find starting endpoint in wall
        search_start = Vec3(pos_start.x, pos_start.y, bridge_z)
        endpoint1 = find_wall_point(cave, search_start, perp * wall_side,
                                     BRIDGE_ENDPOINT_MIN_SDF)
        if endpoint1 is None:
            continue

        # Pick direction to travel along cave (forward or backward)
        travel_dir = rng.choice([-1, 1])

        # Target segment length
        segment_length = rng.uniform(BRIDGE_SEGMENT_LENGTH_MIN, BRIDGE_SEGMENT_LENGTH_MAX)

        # Iteratively build waypoints
        waypoints = [endpoint1]
        consecutive_failures = 0
        last_good_t = t_current
        last_good_side = wall_side * 0.5  # Start inside from wall

        while consecutive_failures < BRIDGE_MAX_CONSECUTIVE_FAILURES:
            # Move along cave
            t_step = segment_length / cave.total_length * travel_dir
            t_next = last_good_t + t_step

            # Stop if we've gone past the cave
            if t_next < 0.0 or t_next > 1.0:
                break

            # Get cave info at new position
            center, hrange, vrange, direction = cave.sample_along_centerline(t_next)
            perp = Vec3(-direction.y, direction.x, 0).normalized()

            # Drift side position slightly (random walk)
            side = last_good_side + rng.uniform(-0.2, 0.2)
            side = max(-0.8, min(0.8, side))

            h_offset = side * hrange

            # Interpolate Z with small variation
            z_variation = rng.uniform(-50, 50)
            waypoint_z = bridge_z + (center.z - pos_start.z) * 0.3 + z_variation

            waypoint = center + perp * h_offset
            waypoint = Vec3(waypoint.x, waypoint.y, waypoint_z)

            # Check if waypoint is inside cave with margin
            if cave.sdf(waypoint) > BRIDGE_WAYPOINT_MAX_SDF:
                consecutive_failures += 1
                continue

            # Success - add waypoint
            waypoints.append(waypoint)
            consecutive_failures = 0
            last_good_t = t_next
            last_good_side = side

        # Need at least BRIDGE_MIN_WAYPOINTS interior waypoints
        if len(waypoints) < BRIDGE_MIN_WAYPOINTS + 1:  # +1 for endpoint1
            continue

        # Find ending endpoint - continue in the direction the bridge was going
        last_wp = waypoints[-1]
        second_last_wp = waypoints[-2] if len(waypoints) >= 2 else endpoint1

        # Use bridge's own direction for termination
        bridge_dir = (last_wp - second_last_wp).normalized()
        # If bridge_dir is zero (degenerate), fall back to cave perpendicular
        if bridge_dir.length() < 0.01:
            _, _, _, end_dir = cave.sample_along_centerline(last_good_t)
            bridge_dir = Vec3(-end_dir.y, end_dir.x, 0).normalized()

        endpoint2 = find_wall_point(cave, last_wp, bridge_dir, BRIDGE_ENDPOINT_MIN_SDF)
        if endpoint2 is None:
            # Try opposite direction
            endpoint2 = find_wall_point(cave, last_wp, bridge_dir * -1, BRIDGE_ENDPOINT_MIN_SDF)
        if endpoint2 is None:
            continue

        waypoints.append(endpoint2)

        # Check collision with existing bridges
        bridge_radius = rng.uniform(BRIDGE_RANGE_MIN, BRIDGE_RANGE_MAX)
        collision = False
        for existing_waypoints, existing_radius in existing_bridges:
            if bridges_collide(waypoints, bridge_radius, existing_waypoints,
                               existing_radius, BRIDGE_COLLISION_MARGIN):
                collision = True
                break

        if collision:
            continue

        # Success! Build the FloodFillPillar
        noise_range = rng.uniform(BRIDGE_NOISE_MIN, BRIDGE_NOISE_MAX)

        points = []
        for wp in waypoints:
            points.append({
                "Location": wp.to_list(),
                "Range": {"Min": bridge_radius * 0.9, "Max": bridge_radius * 1.1},
                "NoiseRange": {"Min": noise_range * 0.9, "Max": noise_range * 1.1},
                "SkewFactor": {"Min": 0.0, "Max": 0.0},
                "FillAmount": {"Min": 100.0, "Max": 100.0}
            })

        return {
            "pillar": {
                "Children": [],
                "Type": "FloodFillPillar",
                "NoiseOverride": None,
                "Points": points,
                "RangeScale": {"Min": 1.0, "Max": 1.0},
                "NoiseRangeScale": {"Min": 1.0, "Max": 1.0},
                "EndcapScale": {"Min": 0.0, "Max": 0.0}
            },
            "waypoints": waypoints,
            "radius": bridge_radius
        }

    return None


def generate_flood_fill_line(rng: random.Random) -> dict:
    """Generate the main cave FloodFillLine."""
    points = []

    x, y, z = CAVE_START_X, CAVE_START_Y, CAVE_START_Z
    heading = 0.0  # Start heading in +X direction

    for i in range(CAVE_NUM_POINTS):
        hrange = rng.uniform(CAVE_HRANGE_MIN, CAVE_HRANGE_MAX)
        vrange = rng.uniform(CAVE_VRANGE_MIN, CAVE_VRANGE_MAX)
        floor_angle = rng.uniform(CAVE_FLOOR_ANGLE_MIN, CAVE_FLOOR_ANGLE_MAX)

        point = {
            "Location": [round(x, 4), round(y, 4), round(z, 4)],
            "HRange": round(hrange, 1),
            "VRange": round(vrange, 1),
            "CielingNoiseRange": CAVE_CEILING_NOISE,
            "WallNoiseRange": CAVE_WALL_NOISE,
            "FloorNoiseRange": CAVE_FLOOR_NOISE,
            "CielingHeight": CAVE_CEILING_HEIGHT,
            "HeightScale": CAVE_HEIGHT_SCALE,
            "FloorDepth": CAVE_FLOOR_DEPTH,
            "FloorAngle": round(floor_angle, 6)
        }
        points.append(point)

        if i < CAVE_NUM_POINTS - 1:
            # Move to next point
            step = rng.uniform(CAVE_STEP_MIN, CAVE_STEP_MAX)
            dx = math.cos(heading) * step
            dy = math.sin(heading) * step
            dz = rng.uniform(CAVE_VERTICAL_STEP_MIN, CAVE_VERTICAL_STEP_MAX)

            x += dx
            y += dy
            z += dz

            # Turn for next step
            heading += rng.uniform(-CAVE_TURN_RATE, CAVE_TURN_RATE)

    return {
        "Children": [],
        "Type": "FloodFillLine",
        "WallNoiseOverride": None,
        "CeilingNoiseOverride": None,
        "FloorNoiseOverride": None,
        "UseDetailNoise": False,
        "Points": points
    }


def calculate_bounds(features: list) -> float:
    """Calculate appropriate bounds based on all feature positions."""
    max_dist = 0.0

    for feature in features:
        if "Points" in feature:
            for point in feature["Points"]:
                loc = point["Location"]
                dist = math.sqrt(loc[0]**2 + loc[1]**2 + loc[2]**2)
                max_dist = max(max_dist, dist)
        if "Location" in feature:
            loc = feature["Location"]
            dist = math.sqrt(loc[0]**2 + loc[1]**2 + loc[2]**2)
            max_dist = max(max_dist, dist)

    # Add margin for cave size and round up
    return math.ceil((max_dist + 4000) / 1000) * 1000


def generate_room(seed: int) -> dict:
    """Generate a complete BigBridge room."""
    rng = random.Random(seed)

    features = []

    # Generate main cave
    cave_line = generate_flood_fill_line(rng)
    features.append(cave_line)

    # Create cave volume for bridge generation
    cave = CaveVolume(cave_line)

    # Generate bridges
    existing_bridges: list[tuple[list[Vec3], float]] = []
    bridges_placed = 0

    for i in range(NUM_BRIDGES):
        result = generate_bridge(rng, cave, existing_bridges)
        if result:
            features.append(result["pillar"])
            existing_bridges.append((result["waypoints"], result["radius"]))
            bridges_placed += 1
        else:
            print(f"Warning: Could not place bridge {i + 1}/{NUM_BRIDGES}", file=sys.stderr)

    print(f"Placed {bridges_placed}/{NUM_BRIDGES} bridges", file=sys.stderr)

    # Calculate bounds
    bounds = calculate_bounds(features)

    return {
        "Base": {
            "Bounds": bounds,
            "CanOnlyBeUsedOnce": False,
            "MirrorSupport": "NotAllowed",
            "RoomTags": []
        },
        "RoomFeatures": features
    }


def main():
    parser = argparse.ArgumentParser(description="Generate a BigBridge room JSON")
    parser.add_argument("--seed", type=int, default=None, help="Random seed (default: random)")
    parser.add_argument("--output", "-o", type=str, default=None,
                        help="Output file (default: stdout)")
    args = parser.parse_args()

    seed = args.seed if args.seed is not None else random.randint(0, 2**31 - 1)
    print(f"Using seed: {seed}", file=sys.stderr)

    room = generate_room(seed)
    output = json.dumps(room, indent=2)

    if args.output:
        with open(args.output, "w") as f:
            f.write(output)
        print(f"Written to {args.output}")
    else:
        print(output)


if __name__ == "__main__":
    main()
