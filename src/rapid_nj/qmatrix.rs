use fixedbitset::FixedBitSet;

use crate::distances::DistanceMatrix;
use crate::rapid_nj::node::Node;
use std::collections::BTreeSet;

struct QMatrix {
    distances: Vec<Option<Vec<f64>>>,
    sum_cols: Vec<Option<f64>>,
    trees: Vec<Option<BTreeSet<Node>>>,
    u_max: f64,
    n: usize,
    n_leaves: usize,
}

impl QMatrix {
    pub fn distance(&self, i: usize, j: usize) -> f64 {
        if i == j {
            0.0
        } else if i < j {
            self.distances[i].as_ref().unwrap()[j - i - 1]
        } else {
            self.distances[j].as_ref().unwrap()[i - j - 1]
        }
    }
    pub fn distances_vec(distances: &Vec<Option<Vec<f64>>>, i: usize, j: usize) -> f64 {
        if i == j {
            0.0
        } else if i < j {
            distances[i].as_ref().unwrap()[j - i - 1]
        } else {
            distances[j].as_ref().unwrap()[i - j - 1]
        }
    }

    pub fn find_neighbors(&self) -> (usize, usize) {
        let mut qmin = f64::INFINITY;
        let mut min_index = (0, 0);
        for (i, tree) in self.trees.iter().enumerate() {
            if tree.is_none() {
                continue;
            }
            let tree = tree.as_ref().unwrap();
            for node in tree.iter() {
                let j = node.index;
                if self.distance(i, j) - self.sum_cols[i].unwrap() - self.u_max >= qmin {
                    break;
                }
                let q = (self.n_leaves as f64 - 2.0) * self.distance(i, j)
                    - self.sum_cols[i].unwrap()
                    - self.sum_cols[j].unwrap();
                if q < qmin {
                    qmin = q;
                    min_index = (i, j);
                }
            }
        }
        min_index
    }
    fn update(&mut self, i: usize, j: usize) {
        self.trees[i] = None;
        self.trees[j] = None;
        self.sum_cols[i] = None;
        self.sum_cols[j] = None;
        let distances = &mut self.distances;
        self.sum_cols.push(Some(0.0));
        for (m, row) in self.trees.iter_mut().enumerate() {
            if row.is_none() {
                continue;
            }
            let row = row.as_mut().unwrap();

            let dim = Self::distances_vec(distances, i, m);
            row.remove(&Node::new(i, dim));
            let djm = Self::distances_vec(distances, j, m);
            row.remove(&Node::new(j, djm));

            let new_distance = 0.5
                * (Self::distances_vec(distances, i, m) + Self::distances_vec(distances, j, m)
                    - Self::distances_vec(distances, i, j));
            row.insert(Node::new(self.n, new_distance));

            self.sum_cols[m] = Some(self.sum_cols[m].unwrap() - dim - djm + new_distance);
            self.sum_cols[self.n] = Some(self.sum_cols[self.n].unwrap() + new_distance);
            distances[m].as_mut().unwrap().push(new_distance);
        }
        self.n_leaves -= 1;
        self.n += 1;
        self.distances.push(Some(Vec::with_capacity(self.n_leaves))); // Maybe add with capacity
        self.distances[i] = None;
        self.distances[j] = None;
        self.trees.push(Some(BTreeSet::new()));
    }
}

// Implement from DistanceMatrix

impl From<&DistanceMatrix> for QMatrix {
    fn from(d: &DistanceMatrix) -> Self {
        let n = d.size();
        let n_leaves = n;
        let matrix = &d.matrix;
        let sum_cols: Vec<Option<f64>> = matrix
            .iter()
            .map(|row| Some(row.iter().sum::<f64>()))
            .collect();
        let u_max = sum_cols
            .iter()
            .max_by(|a, b| a.unwrap().partial_cmp(&b.unwrap()).unwrap())
            .unwrap()
            .unwrap();
        let mut distances = Vec::with_capacity(n);
        for i in 0..n {
            let mut row = Vec::with_capacity(n - i - 1);
            for j in i + 1..n {
                row.push(matrix[i][j]);
            }
            distances.push(Some(row));
        }
        let mut trees = Vec::with_capacity(n);
        for (row_index, row) in distances.iter().enumerate() {
            let mut tree = BTreeSet::new();
            for (col_index, value) in row.as_ref().unwrap().iter().enumerate() {
                tree.insert(Node::new(col_index + row_index + 1, *value));
            }
            trees.push(Some(tree));
        }
        QMatrix {
            distances,
            sum_cols,
            trees,
            u_max,
            n,
            n_leaves,
        }
    }
}

// Test QMatrix::from
#[cfg(test)]
mod tests {
    use super::QMatrix;
    use crate::{distances::DistanceMatrix, rapid_nj::qmatrix::Node};
    #[test]
    fn test_from_distance_matrix() {
        let d = wikipedia_distance_matrix();

        let q = QMatrix::from(&d);
        // Check column sums
        assert_eq!(
            q.sum_cols,
            vec![Some(31.0), Some(34.0), Some(34.0), Some(30.0), Some(27.0)]
        );
        // Check only the upper triangle is stored
        assert_eq!(
            &q.distances,
            &vec![
                Some(vec![5.0, 9.0, 9.0, 8.0]),
                Some(vec![10.0, 10.0, 9.0]),
                Some(vec![8.0, 7.0]),
                Some(vec![3.0]),
                Some(vec![])
            ]
        );
        for i in 0..5 {
            for j in 0..5 {
                assert_eq!(q.distance(i, j), d.matrix[i][j]);
            }
        }
        // Check tree one should be Node(1, 5.0), Node(2, 9.0), Node(3, 9.0)
        let expected_one = vec![
            Node::new(1, 5.0),
            Node::new(4, 8.0),
            Node::new(2, 9.0),
            Node::new(3, 9.0),
        ];
        for (node, expected) in q.trees[0].as_ref().unwrap().iter().zip(expected_one.iter()) {
            assert_eq!(node, expected);
        }
    }
    #[test]
    fn test_find_neighbors() {
        let d = wikipedia_distance_matrix();
        let q = QMatrix::from(&d);
        let neighbors = q.find_neighbors();
        assert_eq!(neighbors, (0, 1));
    }
    #[test]
    fn test_update() {
        let d = wikipedia_distance_matrix();
        let mut q = QMatrix::from(&d);
        q.update(0, 1);
        assert_eq!(
            &q.distances,
            &vec![
                None,
                None,
                Some(vec![8.0, 7.0, 7.0]),
                Some(vec![3.0, 7.0]),
                Some(vec![6.0]),
                Some(vec![])
            ]
        );
        assert_eq!(
            q.sum_cols,
            vec![None, None, Some(22.0), Some(18.0), Some(16.0), Some(20.0)]
        );
        assert_eq!(q.find_neighbors(), (3, 4));
        q.update(3, 4);
        assert_eq!(
            &q.distances,
            &vec![
                None,
                None,
                Some(vec![8.0, 7.0, 7.0, 6.0]),
                None,
                None,
                Some(vec![5.0]),
                Some(vec![])
            ]
        );
    }

    fn wikipedia_distance_matrix() -> DistanceMatrix {
        let d = DistanceMatrix {
            matrix: vec![
                vec![0.0, 5.0, 9.0, 9.0, 8.0],
                vec![5.0, 0.0, 10.0, 10.0, 9.0],
                vec![9.0, 10.0, 0.0, 8.0, 7.0],
                vec![9.0, 10.0, 8.0, 0.0, 3.0],
                vec![8.0, 9.0, 7.0, 3.0, 0.0],
            ],
            names: vec![
                "A".to_string(),
                "B".to_string(),
                "C".to_string(),
                "D".to_string(),
                "E".to_string(),
            ],
        };
        d
    }
}
