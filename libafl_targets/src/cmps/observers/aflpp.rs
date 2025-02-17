use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::{fmt::Debug, marker::PhantomData};

use libafl::{
    executors::ExitKind,
    inputs::UsesInput,
    observers::{
        cmp::{AFLppCmpValuesMetadata, CmpMap, CmpObserver, CmpObserverMetadata, CmpValues},
        Observer,
    },
    state::HasMetadata,
    Error,
};
use libafl_bolts::{ownedref::OwnedRefMut, Named};
use serde::{Deserialize, Serialize};

use crate::cmps::AFLppCmpLogMap;

/* From AFL++ cmplog.h

#define CMP_MAP_W 65536
#define CMP_MAP_H 32
#define CMP_MAP_RTN_H (CMP_MAP_H / 4)

struct cmp_header {

  unsigned hits : 24;
  unsigned id : 24;
  unsigned shape : 5;
  unsigned type : 2;
  unsigned attribute : 4;
  unsigned overflow : 1;
  unsigned reserved : 4;

} __attribute__((packed));

struct cmp_operands {

  u64 v0;
  u64 v1;
  u64 v0_128;
  u64 v1_128;

} __attribute__((packed));

struct cmpfn_operands {

  u8 v0[31];
  u8 v0_len;
  u8 v1[31];
  u8 v1_len;

} __attribute__((packed));

typedef struct cmp_operands cmp_map_list[CMP_MAP_H];

struct cmp_map {

  struct cmp_header   headers[CMP_MAP_W];
  struct cmp_operands log[CMP_MAP_W][CMP_MAP_H];

};
*/

/// A [`CmpObserver`] observer for AFL++ redqueen
#[derive(Serialize, Deserialize, Debug)]
pub struct AFLppCmpLogObserver<'a, S>
where
    S: UsesInput + HasMetadata,
{
    cmp_map: OwnedRefMut<'a, AFLppCmpLogMap>,
    size: Option<OwnedRefMut<'a, usize>>,
    name: String,
    add_meta: bool,
    original: <AFLppCmpValuesMetadata as CmpObserverMetadata<'a, AFLppCmpLogMap>>::Data,
    phantom: PhantomData<S>,
}

impl<'a, S> CmpObserver<'a, AFLppCmpLogMap, S, AFLppCmpValuesMetadata>
    for AFLppCmpLogObserver<'a, S>
where
    S: UsesInput + Debug + HasMetadata,
{
    /// Get the number of usable cmps (all by default)
    fn usable_count(&self) -> usize {
        match &self.size {
            None => self.cmp_map.as_ref().len(),
            Some(o) => *o.as_ref(),
        }
    }

    fn cmp_map(&self) -> &AFLppCmpLogMap {
        self.cmp_map.as_ref()
    }

    fn cmp_map_mut(&mut self) -> &mut AFLppCmpLogMap {
        self.cmp_map.as_mut()
    }

    fn cmp_observer_data(
        &self,
    ) -> <AFLppCmpValuesMetadata as CmpObserverMetadata<'a, AFLppCmpLogMap>>::Data {
        self.original
    }

    /// Add `AFLppCmpValuesMetadata` to the State including the logged values.
    /// This routine does a basic loop filtering because loop index cmps are not interesting.
    fn add_cmpvalues_meta(&mut self, state: &mut S)
    where
        S: HasMetadata,
    {
        #[allow(clippy::option_if_let_else)] // we can't mutate state in a closure
        let meta = if let Some(meta) = state.metadata_map_mut().get_mut::<AFLppCmpValuesMetadata>()
        {
            meta
        } else {
            state.add_metadata(AFLppCmpValuesMetadata::new());
            state
                .metadata_map_mut()
                .get_mut::<AFLppCmpValuesMetadata>()
                .unwrap()
        };

        if self.original {
            // If this observer is for original input, then we have run the un-mutated input
            // Clear orig_cmpvals
            meta.orig_cmpvals.clear();
            // Clear headers
            meta.headers.clear();
        } else {
            // If this observer is for the mutated input
            meta.new_cmpvals.clear();
        }

        let usable_count = self.usable_count();
        let cmp_observer_data = self.cmp_observer_data();

        meta.add_from(usable_count, self.cmp_map_mut(), cmp_observer_data);
    }
}

impl<'a, S> Observer<S> for AFLppCmpLogObserver<'a, S>
where
    S: UsesInput + Debug + HasMetadata,
{
    fn pre_exec(&mut self, _state: &mut S, _input: &S::Input) -> Result<(), Error> {
        self.cmp_map.as_mut().reset()?;
        Ok(())
    }

    fn post_exec(
        &mut self,
        state: &mut S,
        _input: &S::Input,
        _exit_kind: &ExitKind,
    ) -> Result<(), Error> {
        if self.add_meta {
            self.add_cmpvalues_meta(state);
        }
        Ok(())
    }
}

impl<'a, S> Named for AFLppCmpLogObserver<'a, S>
where
    S: UsesInput + HasMetadata,
{
    fn name(&self) -> &str {
        &self.name
    }
}

impl<'a, S> AFLppCmpLogObserver<'a, S>
where
    S: UsesInput + HasMetadata,
{
    /// Creates a new [`AFLppCmpLogObserver`] with the given name and map.
    #[must_use]
    pub fn new(
        name: &'static str,
        cmp_map: OwnedRefMut<'a, AFLppCmpLogMap>,
        add_meta: bool,
    ) -> Self {
        Self {
            name: name.to_string(),
            size: None,
            cmp_map,
            add_meta,
            original: false,
            phantom: PhantomData,
        }
    }
    /// Setter for the flag if the executed input is a mutated one or the original one
    pub fn set_original(&mut self, v: bool) {
        self.original = v;
    }

    /// Creates a new [`AFLppCmpLogObserver`] with the given name, map and reference to variable size.
    #[must_use]
    pub fn with_size(
        name: &'static str,
        cmp_map: OwnedRefMut<'a, AFLppCmpLogMap>,
        add_meta: bool,
        original: bool,
        size: OwnedRefMut<'a, usize>,
    ) -> Self {
        Self {
            name: name.to_string(),
            size: Some(size),
            cmp_map,
            add_meta,
            original,
            phantom: PhantomData,
        }
    }
}

impl<'a> CmpObserverMetadata<'a, AFLppCmpLogMap> for AFLppCmpValuesMetadata {
    type Data = bool;

    fn new_metadata() -> Self {
        Self::new()
    }

    fn add_from(
        &mut self,
        usable_count: usize,
        cmp_map: &mut AFLppCmpLogMap,
        cmp_observer_data: Self::Data,
    ) {
        let count = usable_count;
        for i in 0..count {
            let execs = cmp_map.usable_executions_for(i);
            if execs > 0 {
                if cmp_observer_data {
                    // Update header
                    self.headers.push((i, cmp_map.headers[i]));
                }

                // Recongize loops and discard if needed
                if execs > 4 {
                    let mut increasing_v0 = 0;
                    let mut increasing_v1 = 0;
                    let mut decreasing_v0 = 0;
                    let mut decreasing_v1 = 0;

                    let mut last: Option<CmpValues> = None;
                    for j in 0..execs {
                        if let Some(val) = cmp_map.values_of(i, j) {
                            if let Some(l) = last.and_then(|x| x.to_u64_tuple()) {
                                if let Some(v) = val.to_u64_tuple() {
                                    if l.0.wrapping_add(1) == v.0 {
                                        increasing_v0 += 1;
                                    }
                                    if l.1.wrapping_add(1) == v.1 {
                                        increasing_v1 += 1;
                                    }
                                    if l.0.wrapping_sub(1) == v.0 {
                                        decreasing_v0 += 1;
                                    }
                                    if l.1.wrapping_sub(1) == v.1 {
                                        decreasing_v1 += 1;
                                    }
                                }
                            }
                            last = Some(val);
                        }
                    }
                    // We check for execs-2 because the logged execs may wrap and have something like
                    // 8 9 10 3 4 5 6 7
                    if increasing_v0 >= execs - 2
                        || increasing_v1 >= execs - 2
                        || decreasing_v0 >= execs - 2
                        || decreasing_v1 >= execs - 2
                    {
                        continue;
                    }
                }

                let cmpmap_idx = i;
                let mut cmp_values = Vec::new();
                if cmp_observer_data {
                    // push into orig_cmpvals
                    // println!("Adding to orig_cmpvals");
                    for j in 0..execs {
                        if let Some(val) = cmp_map.values_of(i, j) {
                            cmp_values.push(val);
                        }
                    }
                    // println!("idx: {cmpmap_idx} cmp_values: {:#?}", cmp_values);
                    self.orig_cmpvals.insert(cmpmap_idx, cmp_values);
                } else {
                    // push into new_cmpvals
                    // println!("Adding to new_cmpvals");
                    for j in 0..execs {
                        if let Some(val) = cmp_map.values_of(i, j) {
                            cmp_values.push(val);
                        }
                    }
                    // println!("idx: {cmpmap_idx} cmp_values: {:#?}", cmp_values);
                    self.new_cmpvals.insert(cmpmap_idx, cmp_values);
                }
            }
        }
    }
}
