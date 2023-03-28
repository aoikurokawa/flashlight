use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
};

/// `InputCellId` is a unique identifier for an input cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InputCellId(usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ComputeCellId(usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CallbackId(usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CellId {
    Input(InputCellId),
    Compute(ComputeCellId),
}

impl CellId {
    fn get_id(&self) -> usize {
        match self {
            CellId::Input(id) => id.0,
            CellId::Compute(id) => id.0,
        }
    }
}

struct Callback<'a, T> {
    id: CallbackId,
    function: Box<dyn 'a + FnMut(T)>,
}

type ComputeFunction<T> = Option<Box<dyn Fn(&[T]) -> T>>;

pub struct Cell<'a, T> {
    id: CellId,
    value: T,
    compute_function: ComputeFunction<T>,
    callbacks: HashMap<usize, Callback<'a, T>>,
    callback_id_counter: usize,
    dependencies: Vec<CellId>,
}


impl<'a, T: Copy + PartialEq> Cell<'a, T> {
    fn new(id: CellId, value: T) -> Self {
        Self {
            id,
            value,
            compute_function: None,
            callbacks: HashMap::new(),
            callback_id_counter: 0,
            dependencies: Vec::new(),
        }
    }

    fn new_with_compute(id: CellId, value: T, compute_function: ComputeFunction<T> ) -> Self {
        Self {
            id,
            value,
            compute_function,
            callbacks: HashMap::new(),
            callback_id_counter: 0,
            dependencies: Vec::new(),
        }
    }

    fn set_value(&mut self, new_value: T) {
        self.value = new_value;
    }

    fn add_callback(&mut self, callback: Callback<'a, T>) {
        self.callbacks.insert(callback.id.0, callback);
    }

    fn remove_callback(&mut self, callback_id: usize) -> Option<Callback<T>> {
        self.callbacks.remove(&callback_id)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum RemoveCallbackError {
    NonexistentCell,
    NonexistentCallback,
}

pub struct Reactor<'a, T> {
    cells: Vec<Cell<'a, T>>,
    dependents_map: HashMap<CellId, Vec<CellId>>,
}

impl<'a, T> Default for Reactor<'a, T>
where
    T: Copy + PartialEq,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T> Index<CellId> for Reactor<'a, T>
where
    T: Copy + PartialEq,
{
    type Output = Cell<'a, T>;

    fn index(&self, index: CellId) -> &Self::Output {
        &self.cells[index.get_id()]
    }
}

impl<'a, T> IndexMut<CellId> for Reactor<'a, T>
where
    T: Copy + PartialEq,
{
    fn index_mut(&mut self, index: CellId) -> &mut Self::Output {
        &mut self.cells[index.get_id()]
    }
}

// You are guaranteed that Reactor will only be tested against types that are Copy + PartialEq.
impl<'a, T: Copy + PartialEq> Reactor<'a, T> {
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            dependents_map: HashMap::new(),
        }
    }

    // Creates an input cell with the specified initial value, returning its ID.
    pub fn create_input(&mut self, initial: T) -> InputCellId {
        let id = self.cells.len();
        let input_id = InputCellId(id);
        let cell = Cell::new(CellId::Input(input_id), initial);
        self.cells.push(cell);
        input_id
    }

    // Creates a compute cell with the specified dependencies and compute function.
    // The compute function is expected to take in its arguments in the same order as specified in
    // `dependencies`.
    // You do not need to reject compute functions that expect more arguments than there are
    // dependencies (how would you check for this, anyway?).
    //
    // If any dependency doesn't exist, returns an Err with that nonexistent dependency.
    // (If multiple dependencies do not exist, exactly which one is returned is not defined and
    // will not be tested)
    //
    // Notice that there is no way to *remove* a cell.
    // This means that you may assume, without checking, that if the dependencies exist at creation
    // time they will continue to exist as long as the Reactor exists.
    pub fn create_compute<F: Fn(&[T]) -> T + 'static>(
        &mut self,
        dependencies: &[CellId],
        compute_func: F,
    ) -> Result<ComputeCellId, CellId> {
        let values_result = self.get_value(dependencies);

        match values_result {
            Ok(values) => {
                let cells = &mut self.cells;
                let value = compute_func(&values);
                let id = cells.len();
                let compute_id = ComputeCellId(id);
                let cell_id = CellId::Compute(compute_id);
                let mut cell = Cell::new_with_compute(cell_id, value, Some(Box::new(compute_func)));

                for dependency in dependencies {
                    let dependents = self.dependents_map.entry(*dependency).or_default();

                    dependents.push(cell_id);
                    cell.dependencies.push(*dependency);
                }

                cells.push(cell);

                Ok(compute_id)
            }
            Err(cell_id) => Err(cell_id),
        }
    }

    // Retrieves the current value of the cell, or None if the cell does not exist.
    //
    // You may wonder whether it is possible to implement `get(&self, id: CellId) -> Option<&Cell>`
    // and have a `value(&self)` method on `Cell`.
    //
    // It turns out this introduces a significant amount of extra complexity to this exercise.
    // We chose not to cover this here, since this exercise is probably enough work as-is.
    pub fn value(&self, id: CellId) -> Option<T> {
        self.cells.get(id.get_id()).map(|c| c.value)
    }

    // Sets the value of the specified input cell.
    //
    // Returns false if the cell does not exist.
    //
    // Similarly, you may wonder about `get_mut(&mut self, id: CellId) -> Option<&mut Cell>`, with
    // a `set_value(&mut self, new_value: T)` method on `Cell`.
    //
    // As before, that turned out to add too much extra complexity.
    pub fn set_value(&mut self, id: InputCellId, new_value: T) -> bool {
        match self.cells.get_mut(id.0) {
            Some(cell) => {
                cell.set_value(new_value);
                let mut callback_cells = HashMap::new();
                self.update_dependents(&CellId::Input(id), &mut callback_cells);

                for (callback_cell_id, old_value) in callback_cells {
                    let callback_cell = &mut self[callback_cell_id];

                    if callback_cell.value != old_value {
                        for (_, callback) in callback_cell.callbacks.iter_mut() {
                            (callback.function)(callback_cell.value);
                        }
                    }
                }

                true
            }
            None => false,
        }
    }

    fn update_dependents(&mut self, cell_id: &CellId, callback_cells: &mut HashMap<CellId, T>) {
        if let Some(dependents) = self.dependents_map.clone().get(cell_id) {
            for dependent in dependents {
                let cell = &self[*dependent];
                let computed_cell_id = cell.id;
                let values_result = self.get_value(&cell.dependencies);

                let new_value = cell.compute_function.as_ref().unwrap()(&values_result.unwrap());

                if new_value != cell.value {
                    callback_cells.entry(computed_cell_id).or_insert(cell.value);
                    self[computed_cell_id].value = new_value;
                    self.update_dependents(&computed_cell_id, callback_cells);
                }
            }
        }
    }

    fn get_value(&self, cell_ids: &[CellId]) -> Result<Vec<T>, CellId> {
        let mut values = vec![];
        for id in cell_ids {
            if let Some(value) = self.value(*id) {
                values.push(value);
            } else {
                return Err(*id);
            }
        }

        Ok(values)
    }

    // Adds a callback to the specified compute cell.
    //
    // Returns the ID of the just-added callback, or None if the cell doesn't exist.
    //
    // Callbacks on input cells will not be tested.
    //
    // The semantics of callbacks (as will be tested):
    // For a single set_value call, each compute cell's callbacks should each be called:
    // * Zero times if the compute cell's value did not change as a result of the set_value call.
    // * Exactly once if the compute cell's value changed as a result of the set_value call.
    //   The value passed to the callback should be the final value of the compute cell after the
    //   set_value call.
    pub fn add_callback<F: 'a + FnMut(T)>(
        &mut self,
        id: ComputeCellId,
        callback: F,
    ) -> Option<CallbackId> {
        match self.cells.get_mut(id.0) {
            Some(cell) => {
                let callback_index = cell.callback_id_counter;
                cell.callback_id_counter += 1;

                let callback_id = CallbackId(callback_index);

                let callback = Callback {
                    id: callback_id,
                    function: Box::new(callback),
                };

                cell.add_callback(callback);
                Some(callback_id)
            }
            None => None,
        }
    }

    // Removes the specified callback, using an ID returned from add_callback.
    //
    // Returns an Err if either the cell or callback does not exist.
    //
    // A removed callback should no longer be called.
    pub fn remove_callback(
        &mut self,
        cell: ComputeCellId,
        callback_id: CallbackId,
    ) -> Result<(), RemoveCallbackError> {
        match self.cells.get_mut(cell.0) {
            Some(cell) => {
                let callbacks = &cell.callbacks;

                match callbacks
                    .iter()
                    .find(|(_, callback)| callback.id == callback_id)
                {
                    Some((callback_index, _)) => {
                        cell.remove_callback(*callback_index);
                        Ok(())
                    }
                    None => Err(RemoveCallbackError::NonexistentCallback),
                }
            }
            None => Err(RemoveCallbackError::NonexistentCell),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::react::*;
    #[test]
    fn input_cells_have_a_value() {
        let mut reactor = Reactor::new();
        let input = reactor.create_input(10);
        assert_eq!(reactor.value(CellId::Input(input)), Some(10));
    }

    #[test]
    fn an_input_cells_value_can_be_set() {
        let mut reactor = Reactor::new();
        let input = reactor.create_input(4);
        assert!(reactor.set_value(input, 20));
        assert_eq!(reactor.value(CellId::Input(input)), Some(20));
    }

    #[test]
    fn error_setting_a_nonexistent_input_cell() {
        let mut dummy_reactor = Reactor::new();
        let input = dummy_reactor.create_input(1);
        assert!(!Reactor::new().set_value(input, 0));
    }

    #[test]
    fn compute_cells_calculate_initial_value() {
        let mut reactor = Reactor::new();
        let input = reactor.create_input(1);
        let output = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] + 1)
            .unwrap();
        assert_eq!(reactor.value(CellId::Compute(output)), Some(2));
    }

    #[test]
    fn compute_cells_take_inputs_in_the_right_order() {
        let mut reactor = Reactor::new();
        let one = reactor.create_input(1);
        let two = reactor.create_input(2);
        let output = reactor
            .create_compute(&[CellId::Input(one), CellId::Input(two)], |v| {
                v[0] + v[1] * 10
            })
            .unwrap();
        assert_eq!(reactor.value(CellId::Compute(output)), Some(21));
    }
    #[test]
    fn error_creating_compute_cell_if_input_doesnt_exist() {
        let mut dummy_reactor = Reactor::new();
        let input = dummy_reactor.create_input(1);
        assert_eq!(
            Reactor::new().create_compute(&[CellId::Input(input)], |_| 0),
            Err(CellId::Input(input))
        );
    }
    #[test]
    fn do_not_break_cell_if_creating_compute_cell_with_valid_and_invalid_input() {
        let mut dummy_reactor = Reactor::new();
        let _ = dummy_reactor.create_input(1);
        let dummy_cell = dummy_reactor.create_input(2);
        let mut reactor = Reactor::new();
        let input = reactor.create_input(1);
        assert_eq!(
            reactor.create_compute(&[CellId::Input(input), CellId::Input(dummy_cell)], |_| 0),
            Err(CellId::Input(dummy_cell))
        );
        assert!(reactor.set_value(input, 5));
        assert_eq!(reactor.value(CellId::Input(input)), Some(5));
    }
    #[test]
    fn compute_cells_update_value_when_dependencies_are_changed() {
        let mut reactor = Reactor::new();
        let input = reactor.create_input(1);
        let output = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] + 1)
            .unwrap();
        assert_eq!(reactor.value(CellId::Compute(output)), Some(2));
        assert!(reactor.set_value(input, 3));
        assert_eq!(reactor.value(CellId::Compute(output)), Some(4));
    }
    #[test]
    fn compute_cells_can_depend_on_other_compute_cells() {
        let mut reactor = Reactor::new();
        let input = reactor.create_input(1);
        let times_two = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] * 2)
            .unwrap();
        let times_thirty = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] * 30)
            .unwrap();
        let output = reactor
            .create_compute(
                &[CellId::Compute(times_two), CellId::Compute(times_thirty)],
                |v| v[0] + v[1],
            )
            .unwrap();
        assert_eq!(reactor.value(CellId::Compute(output)), Some(32));
        assert!(reactor.set_value(input, 3));
        assert_eq!(reactor.value(CellId::Compute(output)), Some(96));
    }
    /// A CallbackRecorder helps tests whether callbacks get called correctly.
    /// You'll see it used in tests that deal with callbacks.
    /// The names should be descriptive enough so that the tests make sense,
    /// so it's not necessary to fully understand the implementation,
    /// though you are welcome to.
    struct CallbackRecorder {
        // Note that this `Cell` is https://doc.rust-lang.org/std/cell/
        // a mechanism to allow internal mutability,
        // distinct from the cells (input cells, compute cells) in the reactor
        value: std::cell::Cell<Option<i32>>,
    }
    impl CallbackRecorder {
        fn new() -> Self {
            CallbackRecorder {
                value: std::cell::Cell::new(None),
            }
        }
        fn expect_to_have_been_called_with(&self, v: i32) {
            assert_ne!(
                self.value.get(),
                None,
                "Callback was not called, but should have been"
            );
            assert_eq!(
                self.value.replace(None),
                Some(v),
                "Callback was called with incorrect value"
            );
        }
        fn expect_not_to_have_been_called(&self) {
            assert_eq!(
                self.value.get(),
                None,
                "Callback was called, but should not have been"
            );
        }
        fn callback_called(&self, v: i32) {
            assert_eq!(
                self.value.replace(Some(v)),
                None,
                "Callback was called too many times; can't be called with {v}"
            );
        }
    }
    #[test]
    fn compute_cells_fire_callbacks() {
        let cb = CallbackRecorder::new();
        let mut reactor = Reactor::new();
        let input = reactor.create_input(1);
        let output = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] + 1)
            .unwrap();
        assert!(reactor
            .add_callback(output, |v| cb.callback_called(v))
            .is_some());
        assert!(reactor.set_value(input, 3));
        cb.expect_to_have_been_called_with(4);
    }
    #[test]
    fn error_adding_callback_to_nonexistent_cell() {
        let mut dummy_reactor = Reactor::new();
        let input = dummy_reactor.create_input(1);
        let output = dummy_reactor
            .create_compute(&[CellId::Input(input)], |_| 0)
            .unwrap();
        assert_eq!(
            Reactor::new().add_callback(output, |_: u32| println!("hi")),
            None
        );
    }
    #[test]
    fn error_removing_callback_from_nonexisting_cell() {
        let mut dummy_reactor = Reactor::new();
        let dummy_input = dummy_reactor.create_input(1);
        let _ = dummy_reactor
            .create_compute(&[CellId::Input(dummy_input)], |_| 0)
            .unwrap();
        let dummy_output = dummy_reactor
            .create_compute(&[CellId::Input(dummy_input)], |_| 0)
            .unwrap();
        let mut reactor = Reactor::new();
        let input = reactor.create_input(1);
        let output = reactor
            .create_compute(&[CellId::Input(input)], |_| 0)
            .unwrap();
        let callback = reactor.add_callback(output, |_| ()).unwrap();
        assert_eq!(
            reactor.remove_callback(dummy_output, callback),
            Err(RemoveCallbackError::NonexistentCell)
        );
    }
    #[test]
    fn callbacks_only_fire_on_change() {
        let cb = CallbackRecorder::new();
        let mut reactor = Reactor::new();
        let input = reactor.create_input(1);
        let output = reactor
            .create_compute(
                &[CellId::Input(input)],
                |v| if v[0] < 3 { 111 } else { 222 },
            )
            .unwrap();
        assert!(reactor
            .add_callback(output, |v| cb.callback_called(v))
            .is_some());
        assert!(reactor.set_value(input, 2));
        cb.expect_not_to_have_been_called();
        assert!(reactor.set_value(input, 4));
        cb.expect_to_have_been_called_with(222);
    }
    #[test]
    fn callbacks_can_be_called_multiple_times() {
        let cb = CallbackRecorder::new();
        let mut reactor = Reactor::new();
        let input = reactor.create_input(1);
        let output = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] + 1)
            .unwrap();
        assert!(reactor
            .add_callback(output, |v| cb.callback_called(v))
            .is_some());
        assert!(reactor.set_value(input, 2));
        cb.expect_to_have_been_called_with(3);
        assert!(reactor.set_value(input, 3));
        cb.expect_to_have_been_called_with(4);
    }
    #[test]
    fn callbacks_can_be_called_from_multiple_cells() {
        let cb1 = CallbackRecorder::new();
        let cb2 = CallbackRecorder::new();
        let mut reactor = Reactor::new();
        let input = reactor.create_input(1);
        let plus_one = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] + 1)
            .unwrap();
        let minus_one = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] - 1)
            .unwrap();
        assert!(reactor
            .add_callback(plus_one, |v| cb1.callback_called(v))
            .is_some());
        assert!(reactor
            .add_callback(minus_one, |v| cb2.callback_called(v))
            .is_some());
        assert!(reactor.set_value(input, 10));
        cb1.expect_to_have_been_called_with(11);
        cb2.expect_to_have_been_called_with(9);
    }
    #[test]
    fn callbacks_can_be_added_and_removed() {
        let cb1 = CallbackRecorder::new();
        let cb2 = CallbackRecorder::new();
        let cb3 = CallbackRecorder::new();
        let mut reactor = Reactor::new();
        let input = reactor.create_input(11);
        let output = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] + 1)
            .unwrap();
        let callback = reactor
            .add_callback(output, |v| cb1.callback_called(v))
            .unwrap();
        assert!(reactor
            .add_callback(output, |v| cb2.callback_called(v))
            .is_some());
        assert!(reactor.set_value(input, 31));
        cb1.expect_to_have_been_called_with(32);
        cb2.expect_to_have_been_called_with(32);
        assert!(reactor.remove_callback(output, callback).is_ok());
        assert!(reactor
            .add_callback(output, |v| cb3.callback_called(v))
            .is_some());
        assert!(reactor.set_value(input, 41));
        cb1.expect_not_to_have_been_called();
        cb2.expect_to_have_been_called_with(42);
        cb3.expect_to_have_been_called_with(42);
    }
    #[test]
    fn removing_a_callback_multiple_times_doesnt_interfere_with_other_callbacks() {
        let cb1 = CallbackRecorder::new();
        let cb2 = CallbackRecorder::new();
        let mut reactor = Reactor::new();
        let input = reactor.create_input(1);
        let output = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] + 1)
            .unwrap();
        let callback = reactor
            .add_callback(output, |v| cb1.callback_called(v))
            .unwrap();
        assert!(reactor
            .add_callback(output, |v| cb2.callback_called(v))
            .is_some());
        // We want the first remove to be Ok, but the others should be errors.
        assert!(reactor.remove_callback(output, callback).is_ok());
        for _ in 1..5 {
            assert_eq!(
                reactor.remove_callback(output, callback),
                Err(RemoveCallbackError::NonexistentCallback)
            );
        }
        assert!(reactor.set_value(input, 2));
        cb1.expect_not_to_have_been_called();
        cb2.expect_to_have_been_called_with(3);
    }
    #[test]
    fn callbacks_should_only_be_called_once_even_if_multiple_dependencies_change() {
        let cb = CallbackRecorder::new();
        let mut reactor = Reactor::new();
        let input = reactor.create_input(1);
        let plus_one = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] + 1)
            .unwrap();
        let minus_one1 = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] - 1)
            .unwrap();
        let minus_one2 = reactor
            .create_compute(&[CellId::Compute(minus_one1)], |v| v[0] - 1)
            .unwrap();
        let output = reactor
            .create_compute(
                &[CellId::Compute(plus_one), CellId::Compute(minus_one2)],
                |v| v[0] * v[1],
            )
            .unwrap();
        assert!(reactor
            .add_callback(output, |v| cb.callback_called(v))
            .is_some());
        assert!(reactor.set_value(input, 4));
        cb.expect_to_have_been_called_with(10);
    }
    #[test]
    fn callbacks_should_not_be_called_if_dependencies_change_but_output_value_doesnt_change() {
        let cb = CallbackRecorder::new();
        let mut reactor = Reactor::new();
        let input = reactor.create_input(1);
        let plus_one = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] + 1)
            .unwrap();
        let minus_one = reactor
            .create_compute(&[CellId::Input(input)], |v| v[0] - 1)
            .unwrap();
        let always_two = reactor
            .create_compute(
                &[CellId::Compute(plus_one), CellId::Compute(minus_one)],
                |v| v[0] - v[1],
            )
            .unwrap();
        assert!(reactor
            .add_callback(always_two, |v| cb.callback_called(v))
            .is_some());
        for i in 2..5 {
            assert!(reactor.set_value(input, i));
            cb.expect_not_to_have_been_called();
        }
    }
    #[test]
    fn test_adder_with_boolean_values() {
        // This is a digital logic circuit called an adder:
        // https://en.wikipedia.org/wiki/Adder_(electronics)
        let mut reactor = Reactor::new();
        let a = reactor.create_input(false);
        let b = reactor.create_input(false);
        let carry_in = reactor.create_input(false);
        let a_xor_b = reactor
            .create_compute(&[CellId::Input(a), CellId::Input(b)], |v| v[0] ^ v[1])
            .unwrap();
        let sum = reactor
            .create_compute(&[CellId::Compute(a_xor_b), CellId::Input(carry_in)], |v| {
                v[0] ^ v[1]
            })
            .unwrap();
        let a_xor_b_and_cin = reactor
            .create_compute(&[CellId::Compute(a_xor_b), CellId::Input(carry_in)], |v| {
                v[0] && v[1]
            })
            .unwrap();
        let a_and_b = reactor
            .create_compute(&[CellId::Input(a), CellId::Input(b)], |v| v[0] && v[1])
            .unwrap();
        let carry_out = reactor
            .create_compute(
                &[CellId::Compute(a_xor_b_and_cin), CellId::Compute(a_and_b)],
                |v| v[0] || v[1],
            )
            .unwrap();
        let tests = &[
            (false, false, false, false, false),
            (false, false, true, false, true),
            (false, true, false, false, true),
            (false, true, true, true, false),
            (true, false, false, false, true),
            (true, false, true, true, false),
            (true, true, false, true, false),
            (true, true, true, true, true),
        ];
        for &(aval, bval, cinval, expected_cout, expected_sum) in tests {
            assert!(reactor.set_value(a, aval));
            assert!(reactor.set_value(b, bval));
            assert!(reactor.set_value(carry_in, cinval));
            assert_eq!(reactor.value(CellId::Compute(sum)), Some(expected_sum));
            assert_eq!(
                reactor.value(CellId::Compute(carry_out)),
                Some(expected_cout)
            );
        }
    }
}
