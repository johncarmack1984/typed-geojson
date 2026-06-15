/**
 * The north-star test. Every export below is a function `(x: A) => B` that
 * `return`s its argument unchanged — so it compiles only if `A` is assignable
 * to `B`. We write each pair in both directions, so a green `tsc --noEmit`
 * proves the generated bindings are mutually assignable with `@types/geojson`.
 *
 * Matrix: geometry ∈ {Point, LineString, Polygon, MultiPoint, MultiLineString,
 * MultiPolygon, GeometryCollection, Geometry(union), null, Geometry|null} ×
 * properties ∈ {typed, untyped record, null} × container ∈ {Feature, FC}.
 */
import type * as GeoJSON from "geojson";
import type {
  Bbox,
  Feature,
  FeatureCollection,
  Geometry,
  GeometryCollection,
  Id,
  LineString,
  MultiLineString,
  MultiPoint,
  MultiPolygon,
  Point,
  Polygon,
} from "./bindings";

/** A typical typed-properties struct. */
interface Props {
  name: string;
  population: number;
}

// --- scalars: Bbox <-> BBox, Id <-> string | number --------------------------

export const bboxToNative = (x: Bbox): GeoJSON.BBox => x;
export const bboxFromNative = (x: GeoJSON.BBox): Bbox => x;
export const idToNative = (x: Id): string | number => x;
export const idFromNative = (x: string | number): Id => x;

// --- each named geometry <-> its native counterpart --------------------------

export const pointToNative = (x: Point): GeoJSON.Point => x;
export const pointFromNative = (x: GeoJSON.Point): Point => x;
export const multiPointToNative = (x: MultiPoint): GeoJSON.MultiPoint => x;
export const multiPointFromNative = (x: GeoJSON.MultiPoint): MultiPoint => x;
export const lineToNative = (x: LineString): GeoJSON.LineString => x;
export const lineFromNative = (x: GeoJSON.LineString): LineString => x;
export const multiLineToNative = (x: MultiLineString): GeoJSON.MultiLineString => x;
export const multiLineFromNative = (x: GeoJSON.MultiLineString): MultiLineString => x;
export const polygonToNative = (x: Polygon): GeoJSON.Polygon => x;
export const polygonFromNative = (x: GeoJSON.Polygon): Polygon => x;
export const multiPolygonToNative = (x: MultiPolygon): GeoJSON.MultiPolygon => x;
export const multiPolygonFromNative = (x: GeoJSON.MultiPolygon): MultiPolygon => x;
export const gcToNative = (x: GeometryCollection): GeoJSON.GeometryCollection => x;
export const gcFromNative = (x: GeoJSON.GeometryCollection): GeometryCollection => x;

// --- the geometry union ------------------------------------------------------

export const geometryToNative = (x: Geometry): GeoJSON.Geometry => x;
export const geometryFromNative = (x: GeoJSON.Geometry): Geometry => x;

// --- Feature<G, Props>: typed properties, each geometry ----------------------

export const fPointToNative = (x: Feature<Point, Props>): GeoJSON.Feature<GeoJSON.Point, Props> => x;
export const fPointFromNative = (x: GeoJSON.Feature<GeoJSON.Point, Props>): Feature<Point, Props> => x;
export const fPolygonToNative = (x: Feature<Polygon, Props>): GeoJSON.Feature<GeoJSON.Polygon, Props> => x;
export const fPolygonFromNative = (x: GeoJSON.Feature<GeoJSON.Polygon, Props>): Feature<Polygon, Props> => x;
export const fGeomToNative = (x: Feature<Geometry, Props>): GeoJSON.Feature<GeoJSON.Geometry, Props> => x;
export const fGeomFromNative = (x: GeoJSON.Feature<GeoJSON.Geometry, Props>): Feature<Geometry, Props> => x;

// --- null geometry: the RFC 7946 "unlocated" cases ---------------------------

export const fNullGeomToNative = (x: Feature<null, Props>): GeoJSON.Feature<null, Props> => x;
export const fNullGeomFromNative = (x: GeoJSON.Feature<null, Props>): Feature<null, Props> => x;
export const fNullableToNative = (
  x: Feature<Geometry | null, Props>,
): GeoJSON.Feature<GeoJSON.Geometry | null, Props> => x;
export const fNullableFromNative = (
  x: GeoJSON.Feature<GeoJSON.Geometry | null, Props>,
): Feature<Geometry | null, Props> => x;

// --- properties variations: untyped record, and null ------------------------

export const fUntypedToNative = (
  x: Feature<Point, GeoJSON.GeoJsonProperties>,
): GeoJSON.Feature<GeoJSON.Point, GeoJSON.GeoJsonProperties> => x;
export const fUntypedFromNative = (
  x: GeoJSON.Feature<GeoJSON.Point, GeoJSON.GeoJsonProperties>,
): Feature<Point, GeoJSON.GeoJsonProperties> => x;
export const fNullPropsToNative = (x: Feature<Point, null>): GeoJSON.Feature<GeoJSON.Point, null> => x;
export const fNullPropsFromNative = (x: GeoJSON.Feature<GeoJSON.Point, null>): Feature<Point, null> => x;

// --- FeatureCollection<G, Props> ---------------------------------------------

export const fcPointToNative = (
  x: FeatureCollection<Point, Props>,
): GeoJSON.FeatureCollection<GeoJSON.Point, Props> => x;
export const fcPointFromNative = (
  x: GeoJSON.FeatureCollection<GeoJSON.Point, Props>,
): FeatureCollection<Point, Props> => x;
export const fcGeomToNative = (
  x: FeatureCollection<Geometry, Props>,
): GeoJSON.FeatureCollection<GeoJSON.Geometry, Props> => x;
export const fcGeomFromNative = (
  x: GeoJSON.FeatureCollection<GeoJSON.Geometry, Props>,
): FeatureCollection<Geometry, Props> => x;

// --- negative tests: mismatches MUST fail to compile -------------------------

// @ts-expect-error a Point is not a LineString
export const negPointLine = (x: Point): GeoJSON.LineString => x;
// @ts-expect-error geometry kinds must not cross in a Feature
export const negFeatureGeom = (x: Feature<Point, Props>): GeoJSON.Feature<GeoJSON.LineString, Props> => x;
// @ts-expect-error a non-null geometry Feature is not assignable to the null-geometry form
export const negNonNullToNull = (x: Feature<Point, Props>): GeoJSON.Feature<null, Props> => x;
