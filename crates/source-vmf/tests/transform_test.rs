#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use source_vmf::prelude::*;

    fn transform(origin: &str, angles: &str) -> Transform {
        let mut ent = Entity::new("func_instance", 1);
        ent.set("origin".to_string(), origin.to_string());
        ent.set("angles".to_string(), angles.to_string());
        Transform::from_entity(&ent)
    }

    fn side(plane: &str, u_axis: &str, v_axis: &str) -> Side {
        Side {
            plane: plane.to_string(),
            u_axis: u_axis.to_string(),
            v_axis: v_axis.to_string(),
            ..Default::default()
        }
    }

    fn numbers(value: &str) -> Vec<f64> {
        value
            .split(|c: char| c.is_whitespace() || "()[]".contains(c))
            .filter(|part| !part.is_empty())
            .map(|part| part.parse().unwrap())
            .collect()
    }

    /// Texel that `p` lands on with an axis like `[x y z shift] scale`
    fn texel(axis: &str, p: [f64; 3]) -> f64 {
        let a = numbers(axis);
        (p[0] * a[0] + p[1] * a[1] + p[2] * a[2]) / a[4] + a[3]
    }

    fn normal(plane: &str) -> [f64; 3] {
        let p = numbers(plane);
        let a = [p[0] - p[3], p[1] - p[4], p[2] - p[5]];
        let b = [p[6] - p[3], p[7] - p[4], p[8] - p[5]];
        let n = [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ];
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        n.map(|v| v / len)
    }

    fn near(a: [f64; 3], b: [f64; 3], eps: f64) -> bool {
        a.iter().zip(b).all(|(a, b)| (a - b).abs() < eps)
    }

    #[test]
    fn texels_stay_on_the_same_points() {
        let t = transform("100 -37.5 64", "15 130 -40");
        let local = side(
            "(0 64 64) (64 64 64) (64 0 64)",
            "[0.8 0.6 0 37] 0.25",
            "[0 0 -1 -12] 0.5",
        );
        let mut solid = Solid {
            sides: vec![local.clone()],
            ..Default::default()
        };
        t.apply_solid(&mut solid);
        let moved = &solid.sides[0];

        for p in [[0.0, 64.0, 64.0], [64.0, 0.0, 64.0], [-512.0, 900.0, 64.0]] {
            let w = t.point(p);
            // the turned axis is rounded to 1e-6, that's the whole error budget
            assert!((texel(&moved.u_axis, w) - texel(&local.u_axis, p)).abs() < 0.01);
            assert!((texel(&moved.v_axis, w) - texel(&local.v_axis, p)).abs() < 0.01);
        }
    }

    #[test]
    fn planes_keep_facing_the_same_way() {
        let t = transform("100 -37.5 64", "15 130 -40");
        let mut solid = Solid {
            sides: vec![side(
                "(0 64 64) (64 64 64) (64 0 64)",
                "[1 0 0 0] 0.25",
                "[0 -1 0 0] 0.25",
            )],
            ..Default::default()
        };
        let before = normal(&solid.sides[0].plane);
        t.apply_solid(&mut solid);
        assert!(near(normal(&solid.sides[0].plane), t.vector(before), 1e-5));
    }

    #[test]
    fn displacements_turn_their_vectors() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/vmf_examples/displacements.vmf");
        let vmf = VmfFile::open(path).unwrap();
        let before = vmf
            .world
            .solids
            .iter()
            .find(|solid| solid.sides.iter().any(|side| side.dispinfo.is_some()))
            .unwrap();
        let mut solid = before.clone();
        transform("0 0 0", "0 90 0").apply_solid(&mut solid);

        let i = solid.sides.iter().position(|side| side.dispinfo.is_some()).unwrap();
        let a = before.sides[i].dispinfo.as_ref().unwrap();
        let b = solid.sides[i].dispinfo.as_ref().unwrap();
        assert_eq!(a.start_position, "[-512 0 0]");
        assert_eq!(b.start_position, "[0 -512 0]");
        assert!(a.normals.rows[0].starts_with("0 1 0 0 1 0"));
        assert!(b.normals.rows[0].starts_with("-1 0 0 -1 0 0"));
        assert_eq!(a.normals.rows.len(), b.normals.rows.len());
        assert_eq!(a.distances, b.distances);
        assert_eq!(a.alphas, b.alphas);
    }

    #[test]
    fn entities_move_and_turn() {
        let mut light = Entity::new("light_spot", 1);
        light.set("origin".to_string(), "0 32 0".to_string());
        light.set("angles".to_string(), "-45 10 0".to_string());
        transform("0 0 500", "0 180 0").apply_entity(&mut light);
        assert_eq!(light.get("origin").unwrap(), "0 -32 500");
        assert_eq!(light.get("angles").unwrap(), "-45 -170 0");
    }

    #[test]
    fn unrotated_moves_keep_angles_as_written() {
        let mut prop = Entity::new("prop_static", 1);
        prop.set("origin".to_string(), "1 2 3".to_string());
        prop.set("angles".to_string(), "0 270 0".to_string());
        transform("10 0 0", "0 0 0").apply_entity(&mut prop);
        assert_eq!(prop.get("origin").unwrap(), "11 2 3");
        assert_eq!(prop.get("angles").unwrap(), "0 270 0");

        let mut untouched = prop.clone();
        Transform::IDENTITY.apply_entity(&mut untouched);
        assert_eq!(untouched, prop);
    }

    #[test]
    fn inverse_undoes_the_transform() {
        let t = transform("12 -40 7", "30 115 -20");
        let p = [3.0, 5.0, -8.0];
        assert!(near(t.inverse().point(t.point(p)), p, 1e-5));
        assert!(near((t * t.inverse()).point(p), p, 1e-5));
    }
}
