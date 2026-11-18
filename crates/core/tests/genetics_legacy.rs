// SPDX-License-Identifier: GPL-2.0-or-later
// Expected observations from the pinned tests/fixtures/java/core.jsonl
// Source SHA256: 5f5aa228b3ec67685b3a180a1dca1d42364d099240112bdd33c1ed209a080b10
use biogenesis_core::{GeneticsParams, Genome, GenomeModifiers, JavaRandom};

#[test]
fn complete_inheritance_matches_existing_pinned_java_snapshots() {
    {
        let params = GeneticsParams {
            min_mutation_rate: 0,
            max_mutation_rate: 100,
            meta_mutation_rate: 1000,
            ..GeneticsParams::default()
        };
        let mut random = JavaRandom::new(0);
        let mut genome = Genome::random_with(&mut random, &params).unwrap();
        genome.set_generation(1);
        genome.set_clade_id(Some("oracle-root".into()));
        genome = Genome::inherit_with(&genome, &mut random, &params).unwrap();
        assert_eq!(genome.symmetry(), 7);
        assert_eq!(genome.mirror(), 1);
        assert_eq!(genome.mutation_rate(), 25);
        assert_eq!(genome.activity(), 1);
        assert_eq!(genome.generation(), 2);
        assert_eq!(genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("oracle-root"));
        assert_eq!(genome.update_clade(), false);
        assert_eq!(
            genome.modifiers(),
            GenomeModifiers {
                cream: 2,
                fallow: 4,
                spore: 7,
                adapt_spore: 50,
                plague: false,
                disperse_children: true,
                generation_battle: true,
                sibling_battle: true,
                altruist: true,
                familial: false,
                social: true,
                peaceful: false,
                passive: false,
                clockwise: true,
                mimic_all: false,
                modifies_pink: true,
                modifies_lilac: true,
                modifies_sky: true,
                selfish: false
            }
        );
        assert_eq!(genome.genes().unwrap().len(), 1);
        {
            let gene_handle = genome.gene_ref(0).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x4027672e15196e4f);
            assert_eq!(gene.theta().to_bits(), 0x3fff11b3ef2a52fc);
            assert_eq!(gene.color(), Some(0xff808080));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    3, 2, 5, 5, 5, 4, 5, 2, 4, 3, 5, 2, 1, 0, 2, 5, 2, 4, 3, 4, 0, 5, 5, 5, 5, 5,
                    2, 2, 3, 5, 4, 3, 2, 1, 5, 2
                ]
            );
        }
        genome = Genome::inherit_with(&genome, &mut random, &params).unwrap();
        assert_eq!(genome.symmetry(), 7);
        assert_eq!(genome.mirror(), 1);
        assert_eq!(genome.mutation_rate(), 25);
        assert_eq!(genome.activity(), 1);
        assert_eq!(genome.generation(), 3);
        assert_eq!(genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("oracle-root"));
        assert_eq!(genome.update_clade(), false);
        assert_eq!(
            genome.modifiers(),
            GenomeModifiers {
                cream: 2,
                fallow: 4,
                spore: 7,
                adapt_spore: 50,
                plague: false,
                disperse_children: true,
                generation_battle: true,
                sibling_battle: true,
                altruist: true,
                familial: false,
                social: true,
                peaceful: false,
                passive: false,
                clockwise: true,
                mimic_all: false,
                modifies_pink: true,
                modifies_lilac: true,
                modifies_sky: true,
                selfish: false
            }
        );
        assert_eq!(genome.genes().unwrap().len(), 1);
        {
            let gene_handle = genome.gene_ref(0).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x4027672e15196e4f);
            assert_eq!(gene.theta().to_bits(), 0x3fff11b3ef2a52fc);
            assert_eq!(gene.color(), Some(0xff808080));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    3, 2, 5, 5, 5, 4, 5, 2, 4, 3, 5, 2, 1, 0, 2, 5, 2, 4, 3, 4, 0, 5, 5, 5, 5, 5,
                    2, 2, 3, 5, 4, 3, 2, 1, 5, 2
                ]
            );
        }
        genome = Genome::inherit_with(&genome, &mut random, &params).unwrap();
        assert_eq!(genome.symmetry(), 7);
        assert_eq!(genome.mirror(), 1);
        assert_eq!(genome.mutation_rate(), 25);
        assert_eq!(genome.activity(), 1);
        assert_eq!(genome.generation(), 4);
        assert_eq!(genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("oracle-root"));
        assert_eq!(genome.update_clade(), false);
        assert_eq!(
            genome.modifiers(),
            GenomeModifiers {
                cream: 2,
                fallow: 4,
                spore: 7,
                adapt_spore: 50,
                plague: false,
                disperse_children: true,
                generation_battle: true,
                sibling_battle: true,
                altruist: true,
                familial: false,
                social: true,
                peaceful: false,
                passive: false,
                clockwise: true,
                mimic_all: false,
                modifies_pink: true,
                modifies_lilac: true,
                modifies_sky: true,
                selfish: false
            }
        );
        assert_eq!(genome.genes().unwrap().len(), 1);
        {
            let gene_handle = genome.gene_ref(0).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x4027672e15196e4f);
            assert_eq!(gene.theta().to_bits(), 0x3fff11b3ef2a52fc);
            assert_eq!(gene.color(), Some(0xff808080));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    3, 2, 5, 5, 5, 4, 5, 2, 4, 3, 5, 2, 1, 0, 5, 5, 2, 4, 3, 4, 0, 5, 5, 5, 5, 5,
                    2, 2, 3, 5, 4, 3, 2, 1, 5, 2
                ]
            );
        }
        genome = Genome::inherit_with(&genome, &mut random, &params).unwrap();
        assert_eq!(genome.symmetry(), 7);
        assert_eq!(genome.mirror(), 1);
        assert_eq!(genome.mutation_rate(), 25);
        assert_eq!(genome.activity(), 1);
        assert_eq!(genome.generation(), 5);
        assert_eq!(genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("oracle-root"));
        assert_eq!(genome.update_clade(), false);
        assert_eq!(
            genome.modifiers(),
            GenomeModifiers {
                cream: 2,
                fallow: 4,
                spore: 7,
                adapt_spore: 50,
                plague: false,
                disperse_children: true,
                generation_battle: true,
                sibling_battle: true,
                altruist: true,
                familial: false,
                social: true,
                peaceful: false,
                passive: false,
                clockwise: true,
                mimic_all: false,
                modifies_pink: true,
                modifies_lilac: true,
                modifies_sky: true,
                selfish: false
            }
        );
        assert_eq!(genome.genes().unwrap().len(), 1);
        {
            let gene_handle = genome.gene_ref(0).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x4027672e15196e4f);
            assert_eq!(gene.theta().to_bits(), 0x3fff11b3ef2a52fc);
            assert_eq!(gene.color(), Some(0xff808080));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    3, 2, 5, 5, 5, 4, 5, 2, 4, 3, 5, 2, 1, 0, 5, 5, 2, 4, 3, 4, 0, 5, 5, 5, 5, 5,
                    2, 2, 3, 5, 4, 3, 2, 1, 5, 2
                ]
            );
        }
        assert_eq!(random.next_i64(), -5398001761030105627_i64);
    }
    {
        let params = GeneticsParams {
            min_mutation_rate: 0,
            max_mutation_rate: 100,
            meta_mutation_rate: 1000,
            ..GeneticsParams::default()
        };
        let mut random = JavaRandom::new(42);
        let mut genome = Genome::random_with(&mut random, &params).unwrap();
        genome.set_generation(1);
        genome.set_clade_id(Some("oracle-root".into()));
        genome = Genome::inherit_with(&genome, &mut random, &params).unwrap();
        assert_eq!(genome.symmetry(), 1);
        assert_eq!(genome.mirror(), 1);
        assert_eq!(genome.mutation_rate(), 54);
        assert_eq!(genome.activity(), 2);
        assert_eq!(genome.generation(), 2);
        assert_eq!(genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("oracle-root"));
        assert_eq!(genome.update_clade(), false);
        assert_eq!(
            genome.modifiers(),
            GenomeModifiers {
                cream: 2,
                fallow: 2,
                spore: 1,
                adapt_spore: 4,
                plague: false,
                disperse_children: false,
                generation_battle: true,
                sibling_battle: true,
                altruist: false,
                familial: false,
                social: true,
                peaceful: false,
                passive: false,
                clockwise: true,
                mimic_all: true,
                modifies_pink: true,
                modifies_lilac: true,
                modifies_sky: false,
                selfish: false
            }
        );
        assert_eq!(genome.genes().unwrap().len(), 2);
        {
            let gene_handle = genome.gene_ref(0).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x400622e009e1079c);
            assert_eq!(gene.theta().to_bits(), 0x4017ad48b364193c);
            assert_eq!(gene.color(), Some(0xffffff00));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    1, 5, 2, 2, 0, 0, 2, 2, 0, 0, 1, 5, 2, 1, 2, 3, 5, 3, 4, 4, 1, 0, 4, 0, 2, 5,
                    5, 5, 0, 1, 1, 0, 5, 1, 5, 1
                ]
            );
        }
        {
            let gene_handle = genome.gene_ref(1).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x400b9ae4f8148fbc);
            assert_eq!(gene.theta().to_bits(), 0x40141249828f2086);
            assert_eq!(gene.color(), Some(0xffffc800));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    1, 2, 2, 4, 2, 4, 1, 1, 3, 3, 1, 5, 2, 1, 2, 2, 2, 1, 3, 3, 4, 5, 1, 1, 1, 5,
                    2, 4, 5, 3, 0, 4, 5, 2, 1, 3
                ]
            );
        }
        genome = Genome::inherit_with(&genome, &mut random, &params).unwrap();
        assert_eq!(genome.symmetry(), 1);
        assert_eq!(genome.mirror(), 1);
        assert_eq!(genome.mutation_rate(), 56);
        assert_eq!(genome.activity(), 2);
        assert_eq!(genome.generation(), 3);
        assert_eq!(genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("oracle-root"));
        assert_eq!(genome.update_clade(), false);
        assert_eq!(
            genome.modifiers(),
            GenomeModifiers {
                cream: 2,
                fallow: 2,
                spore: 1,
                adapt_spore: 4,
                plague: false,
                disperse_children: false,
                generation_battle: true,
                sibling_battle: true,
                altruist: false,
                familial: false,
                social: true,
                peaceful: false,
                passive: false,
                clockwise: true,
                mimic_all: true,
                modifies_pink: true,
                modifies_lilac: true,
                modifies_sky: false,
                selfish: false
            }
        );
        assert_eq!(genome.genes().unwrap().len(), 2);
        {
            let gene_handle = genome.gene_ref(0).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x400622e009e1079c);
            assert_eq!(gene.theta().to_bits(), 0x4017ad48b364193c);
            assert_eq!(gene.color(), Some(0xffffff00));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    1, 5, 2, 2, 0, 0, 2, 2, 0, 0, 1, 5, 2, 1, 2, 3, 5, 3, 4, 4, 1, 0, 4, 0, 2, 5,
                    5, 5, 0, 1, 1, 0, 5, 1, 5, 1
                ]
            );
        }
        {
            let gene_handle = genome.gene_ref(1).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x400b9ae4f8148fbc);
            assert_eq!(gene.theta().to_bits(), 0x40141249828f2086);
            assert_eq!(gene.color(), Some(0xffffc800));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    1, 2, 2, 4, 2, 4, 1, 1, 3, 3, 1, 5, 2, 1, 2, 2, 2, 1, 3, 3, 4, 5, 1, 1, 1, 5,
                    2, 4, 5, 3, 0, 4, 5, 2, 1, 3
                ]
            );
        }
        genome = Genome::inherit_with(&genome, &mut random, &params).unwrap();
        assert_eq!(genome.symmetry(), 1);
        assert_eq!(genome.mirror(), 1);
        assert_eq!(genome.mutation_rate(), 56);
        assert_eq!(genome.activity(), 2);
        assert_eq!(genome.generation(), 4);
        assert_eq!(genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("oracle-root"));
        assert_eq!(genome.update_clade(), false);
        assert_eq!(
            genome.modifiers(),
            GenomeModifiers {
                cream: 2,
                fallow: 2,
                spore: 1,
                adapt_spore: 4,
                plague: false,
                disperse_children: false,
                generation_battle: true,
                sibling_battle: true,
                altruist: false,
                familial: false,
                social: true,
                peaceful: false,
                passive: false,
                clockwise: true,
                mimic_all: true,
                modifies_pink: true,
                modifies_lilac: true,
                modifies_sky: false,
                selfish: false
            }
        );
        assert_eq!(genome.genes().unwrap().len(), 2);
        {
            let gene_handle = genome.gene_ref(0).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x400622e009e1079c);
            assert_eq!(gene.theta().to_bits(), 0x4017ad48b364193c);
            assert_eq!(gene.color(), Some(0xffffff00));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    1, 5, 2, 2, 0, 0, 2, 2, 0, 0, 1, 5, 2, 1, 2, 3, 5, 3, 4, 4, 1, 0, 4, 0, 2, 5,
                    5, 5, 0, 1, 1, 0, 5, 1, 5, 1
                ]
            );
        }
        {
            let gene_handle = genome.gene_ref(1).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x400b9ae4f8148fbc);
            assert_eq!(gene.theta().to_bits(), 0x40141249828f2086);
            assert_eq!(gene.color(), Some(0xffffc800));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    1, 2, 2, 4, 2, 4, 1, 1, 3, 3, 1, 5, 2, 1, 2, 2, 2, 1, 3, 3, 4, 5, 1, 1, 1, 5,
                    2, 4, 5, 3, 0, 4, 5, 2, 1, 3
                ]
            );
        }
        genome = Genome::inherit_with(&genome, &mut random, &params).unwrap();
        assert_eq!(genome.symmetry(), 1);
        assert_eq!(genome.mirror(), 1);
        assert_eq!(genome.mutation_rate(), 56);
        assert_eq!(genome.activity(), 2);
        assert_eq!(genome.generation(), 5);
        assert_eq!(genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("oracle-root"));
        assert_eq!(genome.update_clade(), false);
        assert_eq!(
            genome.modifiers(),
            GenomeModifiers {
                cream: 2,
                fallow: 2,
                spore: 1,
                adapt_spore: 4,
                plague: false,
                disperse_children: false,
                generation_battle: true,
                sibling_battle: true,
                altruist: false,
                familial: false,
                social: true,
                peaceful: false,
                passive: false,
                clockwise: true,
                mimic_all: true,
                modifies_pink: true,
                modifies_lilac: true,
                modifies_sky: false,
                selfish: false
            }
        );
        assert_eq!(genome.genes().unwrap().len(), 2);
        {
            let gene_handle = genome.gene_ref(0).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x400622e009e1079c);
            assert_eq!(gene.theta().to_bits(), 0x4017ad48b364193c);
            assert_eq!(gene.color(), Some(0xffffff00));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    1, 5, 2, 2, 0, 0, 2, 2, 0, 0, 1, 5, 2, 1, 2, 3, 5, 3, 4, 4, 1, 0, 4, 0, 2, 5,
                    5, 5, 0, 1, 1, 0, 5, 1, 5, 1
                ]
            );
        }
        {
            let gene_handle = genome.gene_ref(1).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x400b9ae4f8148fbc);
            assert_eq!(gene.theta().to_bits(), 0x40141249828f2086);
            assert_eq!(gene.color(), Some(0xffffc800));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    1, 2, 2, 4, 2, 4, 1, 1, 3, 3, 1, 5, 2, 1, 2, 2, 2, 1, 3, 3, 4, 5, 1, 1, 1, 5,
                    2, 4, 5, 3, 0, 4, 5, 2, 1, 3
                ]
            );
        }
        assert_eq!(random.next_i64(), -3157894548591416132_i64);
    }
    {
        let params = GeneticsParams {
            min_mutation_rate: 0,
            max_mutation_rate: 0,
            meta_mutation_rate: 0,
            ..GeneticsParams::default()
        };
        let mut random = JavaRandom::new(0);
        let mut genome = Genome::random_with(&mut random, &params).unwrap();
        genome.set_generation(1);
        genome.set_clade_id(Some("oracle-root".into()));
        genome = Genome::inherit_with(&genome, &mut random, &params).unwrap();
        assert_eq!(genome.symmetry(), 7);
        assert_eq!(genome.mirror(), 1);
        assert_eq!(genome.mutation_rate(), 0);
        assert_eq!(genome.activity(), 1);
        assert_eq!(genome.generation(), 2);
        assert_eq!(genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("oracle-root"));
        assert_eq!(genome.update_clade(), false);
        assert_eq!(
            genome.modifiers(),
            GenomeModifiers {
                cream: 2,
                fallow: 4,
                spore: 7,
                adapt_spore: 50,
                plague: false,
                disperse_children: true,
                generation_battle: true,
                sibling_battle: true,
                altruist: true,
                familial: false,
                social: true,
                peaceful: false,
                passive: false,
                clockwise: true,
                mimic_all: false,
                modifies_pink: true,
                modifies_lilac: true,
                modifies_sky: true,
                selfish: false
            }
        );
        assert_eq!(genome.genes().unwrap().len(), 1);
        {
            let gene_handle = genome.gene_ref(0).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x4027672e15196e4f);
            assert_eq!(gene.theta().to_bits(), 0x3fff11b3ef2a52fc);
            assert_eq!(gene.color(), Some(0xff808080));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    3, 2, 5, 5, 5, 4, 5, 2, 4, 3, 5, 2, 1, 0, 2, 5, 2, 4, 3, 4, 0, 5, 5, 5, 5, 5,
                    2, 2, 3, 5, 4, 3, 2, 1, 5, 2
                ]
            );
        }
        genome = Genome::inherit_with(&genome, &mut random, &params).unwrap();
        assert_eq!(genome.symmetry(), 7);
        assert_eq!(genome.mirror(), 1);
        assert_eq!(genome.mutation_rate(), 0);
        assert_eq!(genome.activity(), 1);
        assert_eq!(genome.generation(), 3);
        assert_eq!(genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("oracle-root"));
        assert_eq!(genome.update_clade(), false);
        assert_eq!(
            genome.modifiers(),
            GenomeModifiers {
                cream: 2,
                fallow: 4,
                spore: 7,
                adapt_spore: 50,
                plague: false,
                disperse_children: true,
                generation_battle: true,
                sibling_battle: true,
                altruist: true,
                familial: false,
                social: true,
                peaceful: false,
                passive: false,
                clockwise: true,
                mimic_all: false,
                modifies_pink: true,
                modifies_lilac: true,
                modifies_sky: true,
                selfish: false
            }
        );
        assert_eq!(genome.genes().unwrap().len(), 1);
        {
            let gene_handle = genome.gene_ref(0).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x4027672e15196e4f);
            assert_eq!(gene.theta().to_bits(), 0x3fff11b3ef2a52fc);
            assert_eq!(gene.color(), Some(0xff808080));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    3, 2, 5, 5, 5, 4, 5, 2, 4, 3, 5, 2, 1, 0, 2, 5, 2, 4, 3, 4, 0, 5, 5, 5, 5, 5,
                    2, 2, 3, 5, 4, 3, 2, 1, 5, 2
                ]
            );
        }
        genome = Genome::inherit_with(&genome, &mut random, &params).unwrap();
        assert_eq!(genome.symmetry(), 7);
        assert_eq!(genome.mirror(), 1);
        assert_eq!(genome.mutation_rate(), 0);
        assert_eq!(genome.activity(), 1);
        assert_eq!(genome.generation(), 4);
        assert_eq!(genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("oracle-root"));
        assert_eq!(genome.update_clade(), false);
        assert_eq!(
            genome.modifiers(),
            GenomeModifiers {
                cream: 2,
                fallow: 4,
                spore: 7,
                adapt_spore: 50,
                plague: false,
                disperse_children: true,
                generation_battle: true,
                sibling_battle: true,
                altruist: true,
                familial: false,
                social: true,
                peaceful: false,
                passive: false,
                clockwise: true,
                mimic_all: false,
                modifies_pink: true,
                modifies_lilac: true,
                modifies_sky: true,
                selfish: false
            }
        );
        assert_eq!(genome.genes().unwrap().len(), 1);
        {
            let gene_handle = genome.gene_ref(0).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x4027672e15196e4f);
            assert_eq!(gene.theta().to_bits(), 0x3fff11b3ef2a52fc);
            assert_eq!(gene.color(), Some(0xff808080));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    3, 2, 5, 5, 5, 4, 5, 2, 4, 3, 5, 2, 1, 0, 2, 5, 2, 4, 3, 4, 0, 5, 5, 5, 5, 5,
                    2, 2, 3, 5, 4, 3, 2, 1, 5, 2
                ]
            );
        }
        genome = Genome::inherit_with(&genome, &mut random, &params).unwrap();
        assert_eq!(genome.symmetry(), 7);
        assert_eq!(genome.mirror(), 1);
        assert_eq!(genome.mutation_rate(), 0);
        assert_eq!(genome.activity(), 1);
        assert_eq!(genome.generation(), 5);
        assert_eq!(genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("oracle-root"));
        assert_eq!(genome.update_clade(), false);
        assert_eq!(
            genome.modifiers(),
            GenomeModifiers {
                cream: 2,
                fallow: 4,
                spore: 7,
                adapt_spore: 50,
                plague: false,
                disperse_children: true,
                generation_battle: true,
                sibling_battle: true,
                altruist: true,
                familial: false,
                social: true,
                peaceful: false,
                passive: false,
                clockwise: true,
                mimic_all: false,
                modifies_pink: true,
                modifies_lilac: true,
                modifies_sky: true,
                selfish: false
            }
        );
        assert_eq!(genome.genes().unwrap().len(), 1);
        {
            let gene_handle = genome.gene_ref(0).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x4027672e15196e4f);
            assert_eq!(gene.theta().to_bits(), 0x3fff11b3ef2a52fc);
            assert_eq!(gene.color(), Some(0xff808080));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    3, 2, 5, 5, 5, 4, 5, 2, 4, 3, 5, 2, 1, 0, 2, 5, 2, 4, 3, 4, 0, 5, 5, 5, 5, 5,
                    2, 2, 3, 5, 4, 3, 2, 1, 5, 2
                ]
            );
        }
        assert_eq!(random.next_i64(), -1988121919985651200_i64);
    }
    {
        let params = GeneticsParams {
            min_mutation_rate: 0,
            max_mutation_rate: 0,
            meta_mutation_rate: 0,
            ..GeneticsParams::default()
        };
        let mut random = JavaRandom::new(42);
        let mut genome = Genome::random_with(&mut random, &params).unwrap();
        genome.set_generation(1);
        genome.set_clade_id(Some("oracle-root".into()));
        genome = Genome::inherit_with(&genome, &mut random, &params).unwrap();
        assert_eq!(genome.symmetry(), 1);
        assert_eq!(genome.mirror(), 1);
        assert_eq!(genome.mutation_rate(), 0);
        assert_eq!(genome.activity(), 2);
        assert_eq!(genome.generation(), 2);
        assert_eq!(genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("oracle-root"));
        assert_eq!(genome.update_clade(), false);
        assert_eq!(
            genome.modifiers(),
            GenomeModifiers {
                cream: 2,
                fallow: 2,
                spore: 1,
                adapt_spore: 4,
                plague: false,
                disperse_children: false,
                generation_battle: true,
                sibling_battle: true,
                altruist: false,
                familial: false,
                social: true,
                peaceful: false,
                passive: false,
                clockwise: true,
                mimic_all: true,
                modifies_pink: true,
                modifies_lilac: true,
                modifies_sky: false,
                selfish: false
            }
        );
        assert_eq!(genome.genes().unwrap().len(), 2);
        {
            let gene_handle = genome.gene_ref(0).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x400622e009e1079c);
            assert_eq!(gene.theta().to_bits(), 0x4017ad48b364193c);
            assert_eq!(gene.color(), Some(0xffffff00));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    1, 5, 2, 2, 0, 0, 2, 2, 0, 0, 1, 5, 2, 1, 2, 3, 5, 3, 4, 4, 1, 0, 4, 0, 2, 5,
                    5, 5, 0, 1, 1, 0, 5, 1, 5, 1
                ]
            );
        }
        {
            let gene_handle = genome.gene_ref(1).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x400b9ae4f8148fbc);
            assert_eq!(gene.theta().to_bits(), 0x40141249828f2086);
            assert_eq!(gene.color(), Some(0xffffc800));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    1, 2, 2, 4, 2, 4, 1, 1, 3, 3, 1, 5, 2, 1, 2, 2, 2, 1, 3, 3, 4, 5, 1, 1, 1, 5,
                    2, 4, 5, 3, 0, 4, 5, 2, 1, 3
                ]
            );
        }
        genome = Genome::inherit_with(&genome, &mut random, &params).unwrap();
        assert_eq!(genome.symmetry(), 1);
        assert_eq!(genome.mirror(), 1);
        assert_eq!(genome.mutation_rate(), 0);
        assert_eq!(genome.activity(), 2);
        assert_eq!(genome.generation(), 3);
        assert_eq!(genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("oracle-root"));
        assert_eq!(genome.update_clade(), false);
        assert_eq!(
            genome.modifiers(),
            GenomeModifiers {
                cream: 2,
                fallow: 2,
                spore: 1,
                adapt_spore: 4,
                plague: false,
                disperse_children: false,
                generation_battle: true,
                sibling_battle: true,
                altruist: false,
                familial: false,
                social: true,
                peaceful: false,
                passive: false,
                clockwise: true,
                mimic_all: true,
                modifies_pink: true,
                modifies_lilac: true,
                modifies_sky: false,
                selfish: false
            }
        );
        assert_eq!(genome.genes().unwrap().len(), 2);
        {
            let gene_handle = genome.gene_ref(0).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x400622e009e1079c);
            assert_eq!(gene.theta().to_bits(), 0x4017ad48b364193c);
            assert_eq!(gene.color(), Some(0xffffff00));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    1, 5, 2, 2, 0, 0, 2, 2, 0, 0, 1, 5, 2, 1, 2, 3, 5, 3, 4, 4, 1, 0, 4, 0, 2, 5,
                    5, 5, 0, 1, 1, 0, 5, 1, 5, 1
                ]
            );
        }
        {
            let gene_handle = genome.gene_ref(1).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x400b9ae4f8148fbc);
            assert_eq!(gene.theta().to_bits(), 0x40141249828f2086);
            assert_eq!(gene.color(), Some(0xffffc800));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    1, 2, 2, 4, 2, 4, 1, 1, 3, 3, 1, 5, 2, 1, 2, 2, 2, 1, 3, 3, 4, 5, 1, 1, 1, 5,
                    2, 4, 5, 3, 0, 4, 5, 2, 1, 3
                ]
            );
        }
        genome = Genome::inherit_with(&genome, &mut random, &params).unwrap();
        assert_eq!(genome.symmetry(), 1);
        assert_eq!(genome.mirror(), 1);
        assert_eq!(genome.mutation_rate(), 0);
        assert_eq!(genome.activity(), 2);
        assert_eq!(genome.generation(), 4);
        assert_eq!(genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("oracle-root"));
        assert_eq!(genome.update_clade(), false);
        assert_eq!(
            genome.modifiers(),
            GenomeModifiers {
                cream: 2,
                fallow: 2,
                spore: 1,
                adapt_spore: 4,
                plague: false,
                disperse_children: false,
                generation_battle: true,
                sibling_battle: true,
                altruist: false,
                familial: false,
                social: true,
                peaceful: false,
                passive: false,
                clockwise: true,
                mimic_all: true,
                modifies_pink: true,
                modifies_lilac: true,
                modifies_sky: false,
                selfish: false
            }
        );
        assert_eq!(genome.genes().unwrap().len(), 2);
        {
            let gene_handle = genome.gene_ref(0).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x400622e009e1079c);
            assert_eq!(gene.theta().to_bits(), 0x4017ad48b364193c);
            assert_eq!(gene.color(), Some(0xffffff00));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    1, 5, 2, 2, 0, 0, 2, 2, 0, 0, 1, 5, 2, 1, 2, 3, 5, 3, 4, 4, 1, 0, 4, 0, 2, 5,
                    5, 5, 0, 1, 1, 0, 5, 1, 5, 1
                ]
            );
        }
        {
            let gene_handle = genome.gene_ref(1).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x400b9ae4f8148fbc);
            assert_eq!(gene.theta().to_bits(), 0x40141249828f2086);
            assert_eq!(gene.color(), Some(0xffffc800));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    1, 2, 2, 4, 2, 4, 1, 1, 3, 3, 1, 5, 2, 1, 2, 2, 2, 1, 3, 3, 4, 5, 1, 1, 1, 5,
                    2, 4, 5, 3, 0, 4, 5, 2, 1, 3
                ]
            );
        }
        genome = Genome::inherit_with(&genome, &mut random, &params).unwrap();
        assert_eq!(genome.symmetry(), 1);
        assert_eq!(genome.mirror(), 1);
        assert_eq!(genome.mutation_rate(), 0);
        assert_eq!(genome.activity(), 2);
        assert_eq!(genome.generation(), 5);
        assert_eq!(genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("oracle-root"));
        assert_eq!(genome.update_clade(), false);
        assert_eq!(
            genome.modifiers(),
            GenomeModifiers {
                cream: 2,
                fallow: 2,
                spore: 1,
                adapt_spore: 4,
                plague: false,
                disperse_children: false,
                generation_battle: true,
                sibling_battle: true,
                altruist: false,
                familial: false,
                social: true,
                peaceful: false,
                passive: false,
                clockwise: true,
                mimic_all: true,
                modifies_pink: true,
                modifies_lilac: true,
                modifies_sky: false,
                selfish: false
            }
        );
        assert_eq!(genome.genes().unwrap().len(), 2);
        {
            let gene_handle = genome.gene_ref(0).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x400622e009e1079c);
            assert_eq!(gene.theta().to_bits(), 0x4017ad48b364193c);
            assert_eq!(gene.color(), Some(0xffffff00));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    1, 5, 2, 2, 0, 0, 2, 2, 0, 0, 1, 5, 2, 1, 2, 3, 5, 3, 4, 4, 1, 0, 4, 0, 2, 5,
                    5, 5, 0, 1, 1, 0, 5, 1, 5, 1
                ]
            );
        }
        {
            let gene_handle = genome.gene_ref(1).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x400b9ae4f8148fbc);
            assert_eq!(gene.theta().to_bits(), 0x40141249828f2086);
            assert_eq!(gene.color(), Some(0xffffc800));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    1, 2, 2, 4, 2, 4, 1, 1, 3, 3, 1, 5, 2, 1, 2, 2, 2, 1, 3, 3, 4, 5, 1, 1, 1, 5,
                    2, 4, 5, 3, 0, 4, 5, 2, 1, 3
                ]
            );
        }
        assert_eq!(random.next_i64(), -676310601651400508_i64);
    }
    {
        let params = GeneticsParams {
            min_mutation_rate: 10000,
            max_mutation_rate: 10000,
            meta_mutation_rate: 10000,
            ..GeneticsParams::default()
        };
        let mut random = JavaRandom::new(0);
        let mut genome = Genome::random_with(&mut random, &params).unwrap();
        genome.set_generation(1);
        genome.set_clade_id(Some("oracle-root".into()));
        genome = Genome::inherit_with(&genome, &mut random, &params).unwrap();
        assert_eq!(genome.symmetry(), 6);
        assert_eq!(genome.mirror(), 1);
        assert_eq!(genome.mutation_rate(), 10000);
        assert_eq!(genome.activity(), 2);
        assert_eq!(genome.generation(), 2);
        assert_eq!(genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("oracle-root"));
        assert_eq!(genome.update_clade(), true);
        assert_eq!(
            genome.modifiers(),
            GenomeModifiers {
                cream: 3,
                fallow: 4,
                spore: 12,
                adapt_spore: 14,
                plague: false,
                disperse_children: false,
                generation_battle: true,
                sibling_battle: false,
                altruist: false,
                familial: true,
                social: true,
                peaceful: false,
                passive: true,
                clockwise: true,
                mimic_all: true,
                modifies_pink: false,
                modifies_lilac: true,
                modifies_sky: false,
                selfish: true
            }
        );
        assert_eq!(genome.genes().unwrap().len(), 1);
        {
            let gene_handle = genome.gene_ref(0).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x4031e542d943e3f6);
            assert_eq!(gene.theta().to_bits(), 0x401631e0acb7803d);
            assert_eq!(gene.color(), Some(0xffcc7722));
            assert_eq!(gene.branch(), 0);
            assert_eq!(
                *gene.reactions(),
                [
                    3, 2, 2, 1, 4, 2, 4, 2, 4, 0, 0, 5, 3, 5, 1, 4, 5, 2, 4, 3, 1, 5, 5, 5, 1, 4,
                    0, 5, 5, 5, 4, 1, 5, 5, 2, 0
                ]
            );
        }
        genome = Genome::inherit_with(&genome, &mut random, &params).unwrap();
        assert_eq!(genome.symmetry(), 6);
        assert_eq!(genome.mirror(), 0);
        assert_eq!(genome.mutation_rate(), 10000);
        assert_eq!(genome.activity(), 0);
        assert_eq!(genome.generation(), 3);
        assert_eq!(genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("oracle-root"));
        assert_eq!(genome.update_clade(), true);
        assert_eq!(
            genome.modifiers(),
            GenomeModifiers {
                cream: 3,
                fallow: 4,
                spore: 9,
                adapt_spore: 27,
                plague: false,
                disperse_children: true,
                generation_battle: true,
                sibling_battle: false,
                altruist: false,
                familial: false,
                social: false,
                peaceful: false,
                passive: false,
                clockwise: false,
                mimic_all: true,
                modifies_pink: true,
                modifies_lilac: true,
                modifies_sky: true,
                selfish: true
            }
        );
        assert_eq!(genome.genes().unwrap().len(), 1);
        {
            let gene_handle = genome.gene_ref(0).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x402ed294aca3c0e9);
            assert_eq!(gene.theta().to_bits(), 0x3f94106b18f6d9b5);
            assert_eq!(gene.color(), Some(0xff0050a0));
            assert_eq!(gene.branch(), 0);
            assert_eq!(
                *gene.reactions(),
                [
                    0, 2, 2, 5, 2, 4, 1, 4, 0, 3, 0, 4, 3, 3, 1, 1, 0, 0, 0, 1, 2, 5, 2, 1, 5, 4,
                    5, 3, 4, 3, 0, 4, 4, 3, 4, 5
                ]
            );
        }
        genome = Genome::inherit_with(&genome, &mut random, &params).unwrap();
        assert_eq!(genome.symmetry(), 7);
        assert_eq!(genome.mirror(), 1);
        assert_eq!(genome.mutation_rate(), 10000);
        assert_eq!(genome.activity(), 2);
        assert_eq!(genome.generation(), 4);
        assert_eq!(genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("oracle-root"));
        assert_eq!(genome.update_clade(), true);
        assert_eq!(
            genome.modifiers(),
            GenomeModifiers {
                cream: 1,
                fallow: 4,
                spore: 1,
                adapt_spore: 4,
                plague: false,
                disperse_children: true,
                generation_battle: true,
                sibling_battle: true,
                altruist: true,
                familial: false,
                social: false,
                peaceful: true,
                passive: true,
                clockwise: false,
                mimic_all: true,
                modifies_pink: true,
                modifies_lilac: true,
                modifies_sky: true,
                selfish: false
            }
        );
        assert_eq!(genome.genes().unwrap().len(), 1);
        {
            let gene_handle = genome.gene_ref(0).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x400494db873d73e8);
            assert_eq!(gene.theta().to_bits(), 0x4007feb2a53ca0b3);
            assert_eq!(gene.color(), Some(0xff803030));
            assert_eq!(gene.branch(), 0);
            assert_eq!(
                *gene.reactions(),
                [
                    0, 4, 3, 5, 3, 4, 0, 4, 3, 1, 0, 4, 5, 5, 0, 3, 4, 2, 1, 0, 1, 1, 1, 3, 5, 5,
                    3, 0, 0, 1, 1, 4, 0, 3, 1, 3
                ]
            );
        }
        genome = Genome::inherit_with(&genome, &mut random, &params).unwrap();
        assert_eq!(genome.symmetry(), 8);
        assert_eq!(genome.mirror(), 1);
        assert_eq!(genome.mutation_rate(), 10000);
        assert_eq!(genome.activity(), 2);
        assert_eq!(genome.generation(), 5);
        assert_eq!(genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("oracle-root"));
        assert_eq!(genome.update_clade(), true);
        assert_eq!(
            genome.modifiers(),
            GenomeModifiers {
                cream: 2,
                fallow: 1,
                spore: 7,
                adapt_spore: 6,
                plague: false,
                disperse_children: false,
                generation_battle: true,
                sibling_battle: true,
                altruist: false,
                familial: false,
                social: false,
                peaceful: true,
                passive: true,
                clockwise: false,
                mimic_all: true,
                modifies_pink: true,
                modifies_lilac: true,
                modifies_sky: false,
                selfish: false
            }
        );
        assert_eq!(genome.genes().unwrap().len(), 1);
        {
            let gene_handle = genome.gene_ref(0).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x4019af084d663710);
            assert_eq!(gene.theta().to_bits(), 0x400b27ceca859c4a);
            assert_eq!(gene.color(), Some(0xff608040));
            assert_eq!(gene.branch(), 0);
            assert_eq!(
                *gene.reactions(),
                [
                    5, 0, 2, 5, 2, 2, 1, 4, 3, 3, 3, 2, 1, 1, 3, 3, 1, 0, 1, 4, 4, 3, 4, 3, 5, 5,
                    3, 1, 3, 4, 3, 5, 0, 5, 0, 4
                ]
            );
        }
        assert_eq!(random.next_i64(), 4651716431732219016_i64);
    }
    {
        let params = GeneticsParams {
            min_mutation_rate: 10000,
            max_mutation_rate: 10000,
            meta_mutation_rate: 10000,
            ..GeneticsParams::default()
        };
        let mut random = JavaRandom::new(42);
        let mut genome = Genome::random_with(&mut random, &params).unwrap();
        genome.set_generation(1);
        genome.set_clade_id(Some("oracle-root".into()));
        genome = Genome::inherit_with(&genome, &mut random, &params).unwrap();
        assert_eq!(genome.symmetry(), 3);
        assert_eq!(genome.mirror(), 0);
        assert_eq!(genome.mutation_rate(), 10000);
        assert_eq!(genome.activity(), 2);
        assert_eq!(genome.generation(), 2);
        assert_eq!(genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("oracle-root"));
        assert_eq!(genome.update_clade(), true);
        assert_eq!(
            genome.modifiers(),
            GenomeModifiers {
                cream: 1,
                fallow: 3,
                spore: 9,
                adapt_spore: 21,
                plague: false,
                disperse_children: false,
                generation_battle: false,
                sibling_battle: true,
                altruist: true,
                familial: true,
                social: true,
                peaceful: true,
                passive: true,
                clockwise: false,
                mimic_all: true,
                modifies_pink: false,
                modifies_lilac: false,
                modifies_sky: false,
                selfish: true
            }
        );
        assert_eq!(genome.genes().unwrap().len(), 2);
        {
            let gene_handle = genome.gene_ref(0).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x4028adeeafd21675);
            assert_eq!(gene.theta().to_bits(), 0x40135d296f642dda);
            assert_eq!(gene.color(), Some(0xff008080));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    0, 2, 2, 1, 0, 2, 3, 5, 1, 2, 2, 3, 0, 4, 3, 0, 0, 1, 4, 2, 1, 2, 1, 0, 5, 0,
                    3, 0, 3, 2, 3, 2, 4, 0, 5, 5
                ]
            );
        }
        {
            let gene_handle = genome.gene_ref(1).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x40316551ce8ffcc6);
            assert_eq!(gene.theta().to_bits(), 0x4003fb7b126077ef);
            assert_eq!(gene.color(), Some(0xff808080));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    4, 0, 3, 1, 5, 0, 1, 3, 3, 5, 3, 3, 3, 5, 3, 3, 1, 2, 3, 0, 4, 4, 1, 5, 5, 3,
                    0, 2, 0, 0, 1, 2, 3, 0, 4, 3
                ]
            );
        }
        genome = Genome::inherit_with(&genome, &mut random, &params).unwrap();
        assert_eq!(genome.symmetry(), 2);
        assert_eq!(genome.mirror(), 0);
        assert_eq!(genome.mutation_rate(), 10000);
        assert_eq!(genome.activity(), 1);
        assert_eq!(genome.generation(), 3);
        assert_eq!(genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("oracle-root"));
        assert_eq!(genome.update_clade(), true);
        assert_eq!(
            genome.modifiers(),
            GenomeModifiers {
                cream: 2,
                fallow: 1,
                spore: 6,
                adapt_spore: 22,
                plague: true,
                disperse_children: true,
                generation_battle: true,
                sibling_battle: false,
                altruist: false,
                familial: true,
                social: true,
                peaceful: false,
                passive: true,
                clockwise: false,
                mimic_all: true,
                modifies_pink: true,
                modifies_lilac: true,
                modifies_sky: true,
                selfish: true
            }
        );
        assert_eq!(genome.genes().unwrap().len(), 2);
        {
            let gene_handle = genome.gene_ref(0).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x402712e033a7f1a7);
            assert_eq!(gene.theta().to_bits(), 0x40148f5ab5b44922);
            assert_eq!(gene.color(), Some(0xff800000));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    3, 2, 4, 1, 2, 5, 4, 0, 2, 0, 1, 0, 5, 2, 1, 5, 3, 2, 4, 2, 5, 1, 2, 3, 2, 0,
                    4, 1, 1, 1, 5, 4, 5, 4, 5, 0
                ]
            );
        }
        {
            let gene_handle = genome.gene_ref(1).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x402094cbaf18e568);
            assert_eq!(gene.theta().to_bits(), 0x4006daa405e22ad4);
            assert_eq!(gene.color(), Some(0xff008080));
            assert_eq!(gene.branch(), 1);
            assert_eq!(
                *gene.reactions(),
                [
                    4, 1, 4, 1, 3, 3, 3, 3, 0, 3, 3, 2, 3, 0, 2, 1, 1, 0, 2, 0, 0, 3, 3, 4, 5, 2,
                    3, 2, 0, 3, 2, 0, 0, 1, 0, 5
                ]
            );
        }
        genome = Genome::inherit_with(&genome, &mut random, &params).unwrap();
        assert_eq!(genome.symmetry(), 5);
        assert_eq!(genome.mirror(), 0);
        assert_eq!(genome.mutation_rate(), 10000);
        assert_eq!(genome.activity(), 0);
        assert_eq!(genome.generation(), 4);
        assert_eq!(genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("oracle-root"));
        assert_eq!(genome.update_clade(), true);
        assert_eq!(
            genome.modifiers(),
            GenomeModifiers {
                cream: 2,
                fallow: 1,
                spore: 10,
                adapt_spore: 55,
                plague: false,
                disperse_children: false,
                generation_battle: true,
                sibling_battle: false,
                altruist: false,
                familial: true,
                social: true,
                peaceful: false,
                passive: true,
                clockwise: true,
                mimic_all: true,
                modifies_pink: true,
                modifies_lilac: false,
                modifies_sky: true,
                selfish: false
            }
        );
        assert_eq!(genome.genes().unwrap().len(), 2);
        {
            let gene_handle = genome.gene_ref(0).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x40290a52996f7721);
            assert_eq!(gene.theta().to_bits(), 0x401283c961805bae);
            assert_eq!(gene.color(), Some(0xff60c060));
            assert_eq!(gene.branch(), 0);
            assert_eq!(
                *gene.reactions(),
                [
                    1, 4, 1, 1, 1, 5, 5, 0, 2, 2, 3, 2, 3, 4, 1, 3, 5, 1, 5, 3, 0, 2, 0, 2, 5, 5,
                    5, 1, 3, 3, 4, 1, 5, 1, 4, 0
                ]
            );
        }
        {
            let gene_handle = genome.gene_ref(1).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x4027afd4ff806d37);
            assert_eq!(gene.theta().to_bits(), 0x4011cca875fc7906);
            assert_eq!(gene.color(), Some(0xffff00ff));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    4, 1, 5, 2, 5, 1, 4, 4, 2, 3, 1, 4, 1, 3, 1, 4, 1, 4, 4, 1, 1, 0, 3, 1, 0, 2,
                    4, 0, 2, 5, 0, 5, 5, 4, 3, 1
                ]
            );
        }
        genome = Genome::inherit_with(&genome, &mut random, &params).unwrap();
        assert_eq!(genome.symmetry(), 3);
        assert_eq!(genome.mirror(), 0);
        assert_eq!(genome.mutation_rate(), 10000);
        assert_eq!(genome.activity(), 2);
        assert_eq!(genome.generation(), 5);
        assert_eq!(genome.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("oracle-root"));
        assert_eq!(genome.update_clade(), true);
        assert_eq!(
            genome.modifiers(),
            GenomeModifiers {
                cream: 3,
                fallow: 4,
                spore: 2,
                adapt_spore: 69,
                plague: false,
                disperse_children: true,
                generation_battle: false,
                sibling_battle: false,
                altruist: true,
                familial: true,
                social: false,
                peaceful: false,
                passive: true,
                clockwise: false,
                mimic_all: false,
                modifies_pink: true,
                modifies_lilac: false,
                modifies_sky: true,
                selfish: true
            }
        );
        assert_eq!(genome.genes().unwrap().len(), 2);
        {
            let gene_handle = genome.gene_ref(0).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x400eaafc5e7374b0);
            assert_eq!(gene.theta().to_bits(), 0x400d969b8bff7b54);
            assert_eq!(gene.color(), Some(0xff404040));
            assert_eq!(gene.branch(), -1);
            assert_eq!(
                *gene.reactions(),
                [
                    0, 3, 1, 3, 4, 1, 4, 5, 2, 0, 5, 0, 3, 3, 2, 0, 3, 4, 3, 4, 2, 5, 5, 5, 5, 5,
                    1, 1, 5, 4, 1, 1, 4, 4, 1, 1
                ]
            );
        }
        {
            let gene_handle = genome.gene_ref(1).unwrap();
            let gene = gene_handle.borrow();
            assert_eq!(gene.length().to_bits(), 0x4019c4a0a168d0dc);
            assert_eq!(gene.theta().to_bits(), 0x4011d363cb1fefd1);
            assert_eq!(gene.color(), Some(0xff0000ff));
            assert_eq!(gene.branch(), 1);
            assert_eq!(
                *gene.reactions(),
                [
                    3, 0, 2, 1, 4, 1, 2, 2, 0, 4, 5, 1, 2, 4, 0, 4, 4, 3, 1, 4, 1, 3, 0, 5, 3, 1,
                    5, 5, 3, 2, 5, 1, 0, 4, 0, 0
                ]
            );
        }
        assert_eq!(random.next_i64(), -3349173856144723305_i64);
    }
}
