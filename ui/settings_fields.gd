extends RefCounted

const DATA: Dictionary = {
	"tabs": [
		{
			"id": "General",
			"label_key": "T_GENERAL"
		},
		{
			"id": "World",
			"label_key": "T_WORLD"
		},
		{
			"id": "Organisms",
			"label_key": "T_ORGANISMS"
		},
		{
			"id": "Metabolism",
			"label_key": "T_METABOLISM"
		},
		{
			"id": "Genes",
			"label_key": "T_GENES"
		},
		{
			"id": "Genes2",
			"label_key": "T_GENES2"
		},
		{
			"id": "Genes3",
			"label_key": "T_GENES3"
		}
	],
	"fields": [
		{
			"key": "DELAY",
			"fixed_order": 2,
			"source_ui_order": 0,
			"tab": "General",
			"label_keys": [
				"T_TIME_PER_FRAME"
			],
			"unit_key": "T_MILLISECONDS"
		},
		{
			"key": "BACKUP_DELAY",
			"fixed_order": 3,
			"source_ui_order": 1,
			"tab": "General",
			"label_keys": [
				"T_TIME_BETWEEN_BACKUPS"
			]
		},
		{
			"key": "WORLD_WIDTH",
			"fixed_order": 0,
			"source_ui_order": 2,
			"tab": "World",
			"label_keys": [
				"T_WIDTH"
			]
		},
		{
			"key": "WORLD_HEIGHT",
			"fixed_order": 1,
			"source_ui_order": 3,
			"tab": "World",
			"label_keys": [
				"T_HEIGHT"
			]
		},
		{
			"key": "INITIAL_O2",
			"fixed_order": 4,
			"source_ui_order": 4,
			"tab": "World",
			"label_keys": [
				"T_INITIAL_OXYGEN"
			]
		},
		{
			"key": "INITIAL_CO2",
			"fixed_order": 5,
			"source_ui_order": 5,
			"tab": "World",
			"label_keys": [
				"T_INITIAL_CARBON_DIOXIDE"
			]
		},
		{
			"key": "INITIAL_CH4",
			"fixed_order": 6,
			"source_ui_order": 6,
			"tab": "World",
			"label_keys": [
				"T_INITIAL_METHANE"
			]
		},
		{
			"key": "CO2_TO_CH4_DIVISOR",
			"fixed_order": 15,
			"source_ui_order": 7,
			"tab": "World",
			"label_keys": [
				"T_CO2_TO_CH4_DIVISOR"
			]
		},
		{
			"key": "CH4_TO_CO2_DIVISOR",
			"fixed_order": 16,
			"source_ui_order": 8,
			"tab": "World",
			"label_keys": [
				"T_CH4_TO_CO2_DIVISOR"
			]
		},
		{
			"key": "RUBBING",
			"fixed_order": 7,
			"source_ui_order": 9,
			"tab": "World",
			"label_keys": [
				"T_RUBBING_COEFFICIENT"
			]
		},
		{
			"key": "ELASTICITY",
			"fixed_order": 8,
			"source_ui_order": 10,
			"tab": "World",
			"label_keys": [
				"T_ELASTICITY_COEFFICIENT"
			]
		},
		{
			"key": "INITIAL_ORGANISMS",
			"fixed_order": 9,
			"source_ui_order": 11,
			"tab": "Organisms",
			"label_keys": [
				"T_INITIAL_NUMBER"
			]
		},
		{
			"key": "INITIAL_ENERGY",
			"fixed_order": 10,
			"source_ui_order": 12,
			"tab": "Organisms",
			"label_keys": [
				"T_INITIAL_ENERGY"
			]
		},
		{
			"key": "INITIAL_COMPLEXITY",
			"fixed_order": 11,
			"source_ui_order": 13,
			"tab": "Organisms",
			"label_keys": [
				"T_INITIAL_COMPLEXITY"
			]
		},
		{
			"key": "CLADE_COMPLEXITY",
			"fixed_order": 12,
			"source_ui_order": 14,
			"tab": "Organisms",
			"label_keys": [
				"T_CLADE_COMPLEXITY"
			]
		},
		{
			"key": "META_MUTATION_RATE",
			"fixed_order": 17,
			"source_ui_order": 15,
			"tab": "Organisms",
			"label_keys": [
				"T_META_MUTATION_PERCENTAGE"
			]
		},
		{
			"key": "MIN_MUTATION_RATE",
			"fixed_order": 19,
			"source_ui_order": 16,
			"tab": "Organisms",
			"label_keys": [
				"T_MIN_MUTATION_PERCENTAGE"
			]
		},
		{
			"key": "MAX_MUTATION_RATE",
			"fixed_order": 18,
			"source_ui_order": 17,
			"tab": "Organisms",
			"label_keys": [
				"T_MAX_MUTATION_PERCENTAGE"
			]
		},
		{
			"key": "MAX_AGE",
			"fixed_order": 13,
			"source_ui_order": 18,
			"tab": "Organisms",
			"label_keys": [
				"T_LIFE_EXPECTANCY"
			]
		},
		{
			"key": "AGE_DIVISOR",
			"fixed_order": 14,
			"source_ui_order": 19,
			"tab": "Organisms",
			"label_keys": [
				"T_LIFE_EXPECTANCY_DIVISOR"
			]
		},
		{
			"key": "SEGMENT_COST_DIVISOR",
			"fixed_order": 112,
			"source_ui_order": 20,
			"tab": "Organisms",
			"label_keys": [
				"T_UPKEEP_COST_DIVISOR"
			]
		},
		{
			"key": "DRAIN_SUBS_DIVISOR",
			"fixed_order": 113,
			"source_ui_order": 21,
			"tab": "Organisms",
			"label_keys": [
				"T_SUBSTANCE_DRAINAGE_DIVISOR"
			]
		},
		{
			"key": "DECAY_ENERGY",
			"fixed_order": 131,
			"source_ui_order": 22,
			"tab": "Organisms",
			"label_keys": [
				"T_DECAY_ENERGY"
			]
		},
		{
			"key": "GREEN_OBTAINED_ENERGY_DIVISOR",
			"fixed_order": 114,
			"source_ui_order": 23,
			"tab": "Metabolism",
			"label_keys": [
				"T_OBTAINED_ENERGY_DIVISOR"
			]
		},
		{
			"key": "ORGANIC_OBTAINED_ENERGY",
			"fixed_order": 115,
			"source_ui_order": 24,
			"tab": "Metabolism",
			"label_keys": [
				"T_OBTAINED_ENERGY"
			]
		},
		{
			"key": "ORGANIC_SUBS_PRODUCED",
			"fixed_order": 116,
			"source_ui_order": 25,
			"tab": "Metabolism",
			"label_keys": [
				"T_RELEASED_ENERGY_PROPORTION"
			]
		},
		{
			"key": "CREAM_ORGANIC_SUBS_PRODUCED",
			"fixed_order": 117,
			"source_ui_order": 26,
			"tab": "Metabolism",
			"label_keys": [
				"T_CREAM_RELEASED_ENERGY_PROPORTION"
			]
		},
		{
			"key": "HEALING",
			"fixed_order": 118,
			"source_ui_order": 27,
			"tab": "Metabolism",
			"label_keys": [
				"T_HEALING"
			]
		},
		{
			"key": "IMMUNE_SYSTEM",
			"fixed_order": 119,
			"source_ui_order": 28,
			"tab": "Metabolism",
			"label_keys": [
				"T_IMMUNE_SYSTEM"
			]
		},
		{
			"key": "DARKJADE_DELAY",
			"fixed_order": 128,
			"source_ui_order": 29,
			"tab": "Metabolism",
			"label_keys": [
				"T_DARKJADE_DELAY"
			]
		},
		{
			"key": "LAVENDER_SHIELD",
			"fixed_order": 120,
			"source_ui_order": 30,
			"tab": "Metabolism",
			"label_keys": [
				"T_LAVENDER_SHIELD"
			]
		},
		{
			"key": "INDIGO_DIVISOR",
			"fixed_order": 121,
			"source_ui_order": 31,
			"tab": "Metabolism",
			"label_keys": [
				"T_INDIGO_DIVISOR"
			]
		},
		{
			"key": "DODGE_ENERGY_CONSUMPTION",
			"fixed_order": 127,
			"source_ui_order": 32,
			"tab": "Metabolism",
			"label_keys": [
				"T_DODGE_ENERGY_CONSUMPTION"
			]
		},
		{
			"key": "MOSQUITO_ENERGY_CONSUMPTION",
			"fixed_order": 122,
			"source_ui_order": 33,
			"tab": "Metabolism",
			"label_keys": [
				"T_MOSQUITO_ENERGY_CONSUMPTION"
			]
		},
		{
			"key": "EXPERIENCE_ENERGY_CONSUMPTION",
			"fixed_order": 126,
			"source_ui_order": 34,
			"tab": "Metabolism",
			"label_keys": [
				"T_EXPERIENCE_ENERGY_CONSUMPTION"
			]
		},
		{
			"key": "VIRUS_ENERGY_CONSUMPTION",
			"fixed_order": 129,
			"source_ui_order": 35,
			"tab": "Metabolism",
			"label_keys": [
				"T_VIRUS_ENERGY_CONSUMPTION"
			]
		},
		{
			"key": "SCOURGE_ENERGY_CONSUMPTION",
			"fixed_order": 130,
			"source_ui_order": 36,
			"tab": "Metabolism",
			"label_keys": [
				"T_SCOURGE_ENERGY_CONSUMPTION"
			]
		},
		{
			"key": "CROWDEDLIME_ENERGY_CONSUMPTION",
			"fixed_order": 123,
			"source_ui_order": 37,
			"tab": "Metabolism",
			"label_keys": [
				"T_CROWDEDLIME_ENERGY_CONSUMPTION"
			]
		},
		{
			"key": "CROWDEDFOREST_ENERGY_CONSUMPTION",
			"fixed_order": 124,
			"source_ui_order": 38,
			"tab": "Metabolism",
			"label_keys": [
				"T_CROWDEDFOREST_ENERGY_CONSUMPTION"
			]
		},
		{
			"key": "SYMBIONT_ENERGY_CONSUMPTION",
			"fixed_order": 125,
			"source_ui_order": 39,
			"tab": "Metabolism",
			"label_keys": [
				"T_SYMBIONT_ENERGY_CONSUMPTION"
			]
		},
		{
			"key": "GREEN_PROB",
			"fixed_order": 67,
			"source_ui_order": 40,
			"tab": "Genes",
			"label_keys": [
				"T_GREEN",
				"T_PROBABILITY"
			]
		},
		{
			"key": "GREEN_ENERGY_CONSUMPTION",
			"fixed_order": 21,
			"source_ui_order": 41,
			"tab": "Genes",
			"label_keys": [
				"T_GREEN",
				"T_EFFECTIVITY"
			]
		},
		{
			"key": "FOREST_PROB",
			"fixed_order": 77,
			"source_ui_order": 42,
			"tab": "Genes",
			"label_keys": [
				"T_FOREST",
				"T_PROBABILITY"
			]
		},
		{
			"key": "FOREST_ENERGY_CONSUMPTION",
			"fixed_order": 31,
			"source_ui_order": 43,
			"tab": "Genes",
			"label_keys": [
				"T_FOREST",
				"T_EFFECTIVITY"
			]
		},
		{
			"key": "SPRING_PROB",
			"fixed_order": 78,
			"source_ui_order": 44,
			"tab": "Genes",
			"label_keys": [
				"T_SPRING",
				"T_PROBABILITY"
			]
		},
		{
			"key": "SPRING_ENERGY_CONSUMPTION",
			"fixed_order": 32,
			"source_ui_order": 45,
			"tab": "Genes",
			"label_keys": [
				"T_SPRING",
				"T_EFFECTIVITY"
			]
		},
		{
			"key": "SUMMER_PROB",
			"fixed_order": 80,
			"source_ui_order": 46,
			"tab": "Genes",
			"label_keys": [
				"T_SUMMER",
				"T_PROBABILITY"
			]
		},
		{
			"key": "SUMMER_ENERGY_CONSUMPTION",
			"fixed_order": 34,
			"source_ui_order": 47,
			"tab": "Genes",
			"label_keys": [
				"T_SUMMER",
				"T_EFFECTIVITY"
			]
		},
		{
			"key": "LIME_PROB",
			"fixed_order": 81,
			"source_ui_order": 48,
			"tab": "Genes",
			"label_keys": [
				"T_LIME",
				"T_PROBABILITY"
			]
		},
		{
			"key": "LIME_ENERGY_CONSUMPTION",
			"fixed_order": 35,
			"source_ui_order": 49,
			"tab": "Genes",
			"label_keys": [
				"T_LIME",
				"T_EFFECTIVITY"
			]
		},
		{
			"key": "LEAF_PROB",
			"fixed_order": 79,
			"source_ui_order": 50,
			"tab": "Genes",
			"label_keys": [
				"T_LEAF",
				"T_PROBABILITY"
			]
		},
		{
			"key": "LEAF_ENERGY_CONSUMPTION",
			"fixed_order": 33,
			"source_ui_order": 51,
			"tab": "Genes",
			"label_keys": [
				"T_LEAF",
				"T_EFFECTIVITY"
			]
		},
		{
			"key": "C4_PROB",
			"fixed_order": 82,
			"source_ui_order": 52,
			"tab": "Genes",
			"label_keys": [
				"T_C4",
				"T_PROBABILITY"
			]
		},
		{
			"key": "C4_ENERGY_CONSUMPTION",
			"fixed_order": 36,
			"source_ui_order": 53,
			"tab": "Genes",
			"label_keys": [
				"T_C4",
				"T_EFFECTIVITY"
			]
		},
		{
			"key": "JADE_PROB",
			"fixed_order": 83,
			"source_ui_order": 54,
			"tab": "Genes",
			"label_keys": [
				"T_JADE",
				"T_PROBABILITY"
			]
		},
		{
			"key": "JADE_ENERGY_CONSUMPTION",
			"fixed_order": 37,
			"source_ui_order": 55,
			"tab": "Genes",
			"label_keys": [
				"T_JADE",
				"T_EFFECTIVITY"
			]
		},
		{
			"key": "GRASS_PROB",
			"fixed_order": 84,
			"source_ui_order": 56,
			"tab": "Genes",
			"label_keys": [
				"T_GRASS",
				"T_PROBABILITY"
			]
		},
		{
			"key": "GRASS_ENERGY_CONSUMPTION",
			"fixed_order": 38,
			"source_ui_order": 57,
			"tab": "Genes",
			"label_keys": [
				"T_GRASS",
				"T_EFFECTIVITY"
			]
		},
		{
			"key": "BARK_PROB",
			"fixed_order": 86,
			"source_ui_order": 58,
			"tab": "Genes",
			"label_keys": [
				"T_BARK",
				"T_PROBABILITY"
			]
		},
		{
			"key": "BARK_ENERGY_CONSUMPTION",
			"fixed_order": 40,
			"source_ui_order": 59,
			"tab": "Genes",
			"label_keys": [
				"T_BARK",
				"T_EFFECTIVITY"
			]
		},
		{
			"key": "PURPLE_PROB",
			"fixed_order": 85,
			"source_ui_order": 60,
			"tab": "Genes",
			"label_keys": [
				"T_PURPLE",
				"T_PROBABILITY"
			]
		},
		{
			"key": "PURPLE_ENERGY_CONSUMPTION",
			"fixed_order": 39,
			"source_ui_order": 61,
			"tab": "Genes",
			"label_keys": [
				"T_PURPLE",
				"T_EFFECTIVITY"
			]
		},
		{
			"key": "DARKGRAY_PROB",
			"fixed_order": 101,
			"source_ui_order": 62,
			"tab": "Genes",
			"label_keys": [
				"T_DARKGRAY",
				"T_PROBABILITY"
			]
		},
		{
			"key": "DARKGRAY_ENERGY_CONSUMPTION",
			"fixed_order": 55,
			"source_ui_order": 63,
			"tab": "Genes",
			"label_keys": [
				"T_DARKGRAY",
				"T_EFFECTIVITY"
			]
		},
		{
			"key": "RED_PROB",
			"fixed_order": 66,
			"source_ui_order": 64,
			"tab": "Genes2",
			"label_keys": [
				"T_RED",
				"T_PROBABILITY"
			]
		},
		{
			"key": "RED_ENERGY_CONSUMPTION",
			"fixed_order": 20,
			"source_ui_order": 65,
			"tab": "Genes2",
			"label_keys": [
				"T_RED",
				"T_COST"
			]
		},
		{
			"key": "FIRE_PROB",
			"fixed_order": 100,
			"source_ui_order": 66,
			"tab": "Genes2",
			"label_keys": [
				"T_FIRE",
				"T_PROBABILITY"
			]
		},
		{
			"key": "FIRE_ENERGY_CONSUMPTION",
			"fixed_order": 54,
			"source_ui_order": 67,
			"tab": "Genes2",
			"label_keys": [
				"T_FIRE",
				"T_COST"
			]
		},
		{
			"key": "ORANGE_PROB",
			"fixed_order": 76,
			"source_ui_order": 68,
			"tab": "Genes2",
			"label_keys": [
				"T_ORANGE",
				"T_PROBABILITY"
			]
		},
		{
			"key": "ORANGE_ENERGY_CONSUMPTION",
			"fixed_order": 30,
			"source_ui_order": 69,
			"tab": "Genes2",
			"label_keys": [
				"T_ORANGE",
				"T_COST"
			]
		},
		{
			"key": "MAROON_PROB",
			"fixed_order": 90,
			"source_ui_order": 70,
			"tab": "Genes2",
			"label_keys": [
				"T_MAROON",
				"T_PROBABILITY"
			]
		},
		{
			"key": "MAROON_ENERGY_CONSUMPTION",
			"fixed_order": 44,
			"source_ui_order": 71,
			"tab": "Genes2",
			"label_keys": [
				"T_MAROON",
				"T_COST"
			]
		},
		{
			"key": "PINK_PROB",
			"fixed_order": 74,
			"source_ui_order": 72,
			"tab": "Genes2",
			"label_keys": [
				"T_PINK",
				"T_PROBABILITY"
			]
		},
		{
			"key": "PINK_ENERGY_CONSUMPTION",
			"fixed_order": 28,
			"source_ui_order": 73,
			"tab": "Genes2",
			"label_keys": [
				"T_PINK",
				"T_COST"
			]
		},
		{
			"key": "CREAM_PROB",
			"fixed_order": 93,
			"source_ui_order": 74,
			"tab": "Genes2",
			"label_keys": [
				"T_CREAM",
				"T_PROBABILITY"
			]
		},
		{
			"key": "CREAM_ENERGY_CONSUMPTION",
			"fixed_order": 47,
			"source_ui_order": 75,
			"tab": "Genes2",
			"label_keys": [
				"T_CREAM",
				"T_COST"
			]
		},
		{
			"key": "SILVER_PROB",
			"fixed_order": 99,
			"source_ui_order": 76,
			"tab": "Genes2",
			"label_keys": [
				"T_SILVER",
				"T_PROBABILITY"
			]
		},
		{
			"key": "SILVER_ENERGY_CONSUMPTION",
			"fixed_order": 53,
			"source_ui_order": 77,
			"tab": "Genes2",
			"label_keys": [
				"T_SILVER",
				"T_COST"
			]
		},
		{
			"key": "SPIKE_PROB",
			"fixed_order": 107,
			"source_ui_order": 78,
			"tab": "Genes2",
			"label_keys": [
				"T_SPIKE",
				"T_PROBABILITY"
			]
		},
		{
			"key": "SPIKE_ENERGY_CONSUMPTION",
			"fixed_order": 61,
			"source_ui_order": 79,
			"tab": "Genes2",
			"label_keys": [
				"T_SPIKE",
				"T_COST"
			]
		},
		{
			"key": "LILAC_PROB",
			"fixed_order": 98,
			"source_ui_order": 80,
			"tab": "Genes2",
			"label_keys": [
				"T_LILAC",
				"T_PROBABILITY"
			]
		},
		{
			"key": "LILAC_ENERGY_CONSUMPTION",
			"fixed_order": 52,
			"source_ui_order": 81,
			"tab": "Genes2",
			"label_keys": [
				"T_LILAC",
				"T_COST"
			]
		},
		{
			"key": "GRAY_PROB",
			"fixed_order": 71,
			"source_ui_order": 82,
			"tab": "Genes2",
			"label_keys": [
				"T_GRAY",
				"T_PROBABILITY"
			]
		},
		{
			"key": "GRAY_ENERGY_CONSUMPTION",
			"fixed_order": 25,
			"source_ui_order": 83,
			"tab": "Genes2",
			"label_keys": [
				"T_GRAY",
				"T_COST"
			]
		},
		{
			"key": "VIOLET_PROB",
			"fixed_order": 87,
			"source_ui_order": 84,
			"tab": "Genes2",
			"label_keys": [
				"T_VIOLET",
				"T_PROBABILITY"
			]
		},
		{
			"key": "VIOLET_ENERGY_CONSUMPTION",
			"fixed_order": 41,
			"source_ui_order": 85,
			"tab": "Genes2",
			"label_keys": [
				"T_VIOLET",
				"T_COST"
			]
		},
		{
			"key": "OLIVE_PROB",
			"fixed_order": 91,
			"source_ui_order": 86,
			"tab": "Genes2",
			"label_keys": [
				"T_OLIVE",
				"T_PROBABILITY"
			]
		},
		{
			"key": "OLIVE_ENERGY_CONSUMPTION",
			"fixed_order": 45,
			"source_ui_order": 87,
			"tab": "Genes2",
			"label_keys": [
				"T_OLIVE",
				"T_COST"
			]
		},
		{
			"key": "SKY_PROB",
			"fixed_order": 97,
			"source_ui_order": 88,
			"tab": "Genes2",
			"label_keys": [
				"T_SKY",
				"T_PROBABILITY"
			]
		},
		{
			"key": "SKY_ENERGY_CONSUMPTION",
			"fixed_order": 51,
			"source_ui_order": 89,
			"tab": "Genes2",
			"label_keys": [
				"T_SKY",
				"T_COST"
			]
		},
		{
			"key": "BLUE_PROB",
			"fixed_order": 68,
			"source_ui_order": 90,
			"tab": "Genes2",
			"label_keys": [
				"T_BLUE",
				"T_PROBABILITY"
			]
		},
		{
			"key": "BLUE_ENERGY_CONSUMPTION",
			"fixed_order": 22,
			"source_ui_order": 91,
			"tab": "Genes2",
			"label_keys": [
				"T_BLUE",
				"T_COST"
			]
		},
		{
			"key": "OCHRE_PROB",
			"fixed_order": 96,
			"source_ui_order": 92,
			"tab": "Genes2",
			"label_keys": [
				"T_OCHRE",
				"T_PROBABILITY"
			]
		},
		{
			"key": "OCHRE_ENERGY_CONSUMPTION",
			"fixed_order": 50,
			"source_ui_order": 93,
			"tab": "Genes2",
			"label_keys": [
				"T_OCHRE",
				"T_COST"
			]
		},
		{
			"key": "FALLOW_PROB",
			"fixed_order": 110,
			"source_ui_order": 94,
			"tab": "Genes2",
			"label_keys": [
				"T_FALLOW",
				"T_PROBABILITY"
			]
		},
		{
			"key": "FALLOW_ENERGY_CONSUMPTION",
			"fixed_order": 64,
			"source_ui_order": 95,
			"tab": "Genes2",
			"label_keys": [
				"T_FALLOW",
				"T_COST"
			]
		},
		{
			"key": "SPORE_PROB",
			"fixed_order": 111,
			"source_ui_order": 96,
			"tab": "Genes2",
			"label_keys": [
				"T_SPORE",
				"T_PROBABILITY"
			]
		},
		{
			"key": "SPORE_ENERGY_CONSUMPTION",
			"fixed_order": 65,
			"source_ui_order": 97,
			"tab": "Genes2",
			"label_keys": [
				"T_SPORE",
				"T_COST"
			]
		},
		{
			"key": "WHITE_PROB",
			"fixed_order": 70,
			"source_ui_order": 98,
			"tab": "Genes3",
			"label_keys": [
				"T_WHITE",
				"T_PROBABILITY"
			]
		},
		{
			"key": "WHITE_ENERGY_CONSUMPTION",
			"fixed_order": 24,
			"source_ui_order": 99,
			"tab": "Genes3",
			"label_keys": [
				"T_WHITE",
				"T_COST"
			]
		},
		{
			"key": "PLAGUE_PROB",
			"fixed_order": 106,
			"source_ui_order": 100,
			"tab": "Genes3",
			"label_keys": [
				"T_PLAGUE",
				"T_PROBABILITY"
			]
		},
		{
			"key": "PLAGUE_ENERGY_CONSUMPTION",
			"fixed_order": 60,
			"source_ui_order": 101,
			"tab": "Genes3",
			"label_keys": [
				"T_PLAGUE",
				"T_COST"
			]
		},
		{
			"key": "CORAL_PROB",
			"fixed_order": 75,
			"source_ui_order": 102,
			"tab": "Genes3",
			"label_keys": [
				"T_CORAL",
				"T_PROBABILITY"
			]
		},
		{
			"key": "CORAL_ENERGY_CONSUMPTION",
			"fixed_order": 29,
			"source_ui_order": 103,
			"tab": "Genes3",
			"label_keys": [
				"T_CORAL",
				"T_COST"
			]
		},
		{
			"key": "MINT_PROB",
			"fixed_order": 92,
			"source_ui_order": 104,
			"tab": "Genes3",
			"label_keys": [
				"T_MINT",
				"T_PROBABILITY"
			]
		},
		{
			"key": "MINT_ENERGY_CONSUMPTION",
			"fixed_order": 46,
			"source_ui_order": 105,
			"tab": "Genes3",
			"label_keys": [
				"T_MINT",
				"T_COST"
			]
		},
		{
			"key": "LAVENDER_PROB",
			"fixed_order": 109,
			"source_ui_order": 106,
			"tab": "Genes3",
			"label_keys": [
				"T_LAVENDER",
				"T_PROBABILITY"
			]
		},
		{
			"key": "LAVENDER_ENERGY_CONSUMPTION",
			"fixed_order": 63,
			"source_ui_order": 107,
			"tab": "Genes3",
			"label_keys": [
				"T_LAVENDER",
				"T_COST"
			]
		},
		{
			"key": "MAGENTA_PROB",
			"fixed_order": 73,
			"source_ui_order": 108,
			"tab": "Genes3",
			"label_keys": [
				"T_MAGENTA",
				"T_PROBABILITY"
			]
		},
		{
			"key": "MAGENTA_ENERGY_CONSUMPTION",
			"fixed_order": 27,
			"source_ui_order": 109,
			"tab": "Genes3",
			"label_keys": [
				"T_MAGENTA",
				"T_COST"
			]
		},
		{
			"key": "ROSE_PROB",
			"fixed_order": 94,
			"source_ui_order": 110,
			"tab": "Genes3",
			"label_keys": [
				"T_ROSE",
				"T_PROBABILITY"
			]
		},
		{
			"key": "ROSE_ENERGY_CONSUMPTION",
			"fixed_order": 48,
			"source_ui_order": 111,
			"tab": "Genes3",
			"label_keys": [
				"T_ROSE",
				"T_COST"
			]
		},
		{
			"key": "CYAN_PROB",
			"fixed_order": 69,
			"source_ui_order": 112,
			"tab": "Genes3",
			"label_keys": [
				"T_CYAN",
				"T_PROBABILITY"
			]
		},
		{
			"key": "CYAN_ENERGY_CONSUMPTION",
			"fixed_order": 23,
			"source_ui_order": 113,
			"tab": "Genes3",
			"label_keys": [
				"T_CYAN",
				"T_COST"
			]
		},
		{
			"key": "TEAL_PROB",
			"fixed_order": 88,
			"source_ui_order": 114,
			"tab": "Genes3",
			"label_keys": [
				"T_TEAL",
				"T_PROBABILITY"
			]
		},
		{
			"key": "TEAL_ENERGY_CONSUMPTION",
			"fixed_order": 42,
			"source_ui_order": 115,
			"tab": "Genes3",
			"label_keys": [
				"T_TEAL",
				"T_COST"
			]
		},
		{
			"key": "YELLOW_PROB",
			"fixed_order": 72,
			"source_ui_order": 116,
			"tab": "Genes3",
			"label_keys": [
				"T_YELLOW",
				"T_PROBABILITY"
			]
		},
		{
			"key": "YELLOW_ENERGY_CONSUMPTION",
			"fixed_order": 26,
			"source_ui_order": 117,
			"tab": "Genes3",
			"label_keys": [
				"T_YELLOW",
				"T_COST"
			]
		},
		{
			"key": "AUBURN_PROB",
			"fixed_order": 105,
			"source_ui_order": 118,
			"tab": "Genes3",
			"label_keys": [
				"T_AUBURN",
				"T_PROBABILITY"
			]
		},
		{
			"key": "AUBURN_ENERGY_CONSUMPTION",
			"fixed_order": 59,
			"source_ui_order": 119,
			"tab": "Genes3",
			"label_keys": [
				"T_AUBURN",
				"T_COST"
			]
		},
		{
			"key": "INDIGO_PROB",
			"fixed_order": 108,
			"source_ui_order": 120,
			"tab": "Genes3",
			"label_keys": [
				"T_INDIGO",
				"T_PROBABILITY"
			]
		},
		{
			"key": "INDIGO_ENERGY_CONSUMPTION",
			"fixed_order": 62,
			"source_ui_order": 121,
			"tab": "Genes3",
			"label_keys": [
				"T_INDIGO",
				"T_COST"
			]
		},
		{
			"key": "BLOND_PROB",
			"fixed_order": 103,
			"source_ui_order": 122,
			"tab": "Genes3",
			"label_keys": [
				"T_BLOND",
				"T_PROBABILITY"
			]
		},
		{
			"key": "BLOND_ENERGY_CONSUMPTION",
			"fixed_order": 57,
			"source_ui_order": 123,
			"tab": "Genes3",
			"label_keys": [
				"T_BLOND",
				"T_COST"
			]
		},
		{
			"key": "FLOWER_PROB",
			"fixed_order": 104,
			"source_ui_order": 124,
			"tab": "Genes3",
			"label_keys": [
				"T_FLOWER",
				"T_PROBABILITY"
			]
		},
		{
			"key": "FLOWER_ENERGY_CONSUMPTION",
			"fixed_order": 58,
			"source_ui_order": 125,
			"tab": "Genes3",
			"label_keys": [
				"T_FLOWER",
				"T_COST"
			]
		},
		{
			"key": "GOLD_PROB",
			"fixed_order": 102,
			"source_ui_order": 126,
			"tab": "Genes3",
			"label_keys": [
				"T_GOLD",
				"T_PROBABILITY"
			]
		},
		{
			"key": "GOLD_DIVISOR",
			"fixed_order": 56,
			"source_ui_order": 127,
			"tab": "Genes3",
			"label_keys": [
				"T_GOLD",
				"T_COST"
			]
		},
		{
			"key": "DARK_PROB",
			"fixed_order": 95,
			"source_ui_order": 128,
			"tab": "Genes3",
			"label_keys": [
				"T_DARK",
				"T_PROBABILITY"
			]
		},
		{
			"key": "DARK_ENERGY_CONSUMPTION",
			"fixed_order": 49,
			"source_ui_order": 129,
			"tab": "Genes3",
			"label_keys": [
				"T_DARK",
				"T_COST"
			]
		},
		{
			"key": "EYE_PROB",
			"fixed_order": 89,
			"source_ui_order": 130,
			"tab": "Genes3",
			"label_keys": [
				"T_EYE",
				"T_PROBABILITY"
			]
		},
		{
			"key": "EYE_ENERGY_CONSUMPTION",
			"fixed_order": 43,
			"source_ui_order": 131,
			"tab": "Genes3",
			"label_keys": [
				"T_EYE",
				"T_COST"
			]
		}
	],
	"messages": {
		"en": {
			"T_AUBURN": "auburn",
			"T_AUTOMATIC_BACKUPS": "Automatic Backups",
			"T_BARK": "bark",
			"T_BLOND": "blond",
			"T_BLUE": "blue",
			"T_C4": "c4plant",
			"T_CANCEL": "Cancel",
			"T_CATALAN": "Catalan",
			"T_CH4_TO_CO2_DIVISOR": "CH4 to CO2 divisor:",
			"T_CHEMOORGANOTROPHIC_METABOLISM": "Chemoorganotrophic metabolism:",
			"T_CLADE_COMPLEXITY": "Clade complexity:",
			"T_CO2_TO_CH4_DIVISOR": "CO2 to CH4 divisor:",
			"T_COLOR2": "Color",
			"T_CORAL": "coral",
			"T_COST": "Cost:",
			"T_CREAM": "cream",
			"T_CREAM_RELEASED_ENERGY_PROPORTION": "Cream released energy proportion:",
			"T_CROWDEDFOREST_ENERGY_CONSUMPTION": "+ Forest effectivity if not alone:",
			"T_CROWDEDLIME_ENERGY_CONSUMPTION": "Lime effectivity if not alone:",
			"T_CYAN": "cyan",
			"T_DARK": "black",
			"T_DARKGRAY": "dark gray",
			"T_DARKJADE_DELAY": "Jade healing delay:",
			"T_DECAY_ENERGY": "Energy lost by dead organisms: ",
			"T_DEFAULT_VALUES": "Default values",
			"T_DISABLE_FBOBJECT": "Disable fbobject (try if OpenGL doesn't work)",
			"T_DODGE_ENERGY_CONSUMPTION": "Dodge cost:",
			"T_EFFECTIVITY": "Effectivity:",
			"T_ELASTICITY_COEFFICIENT": "Elasticity coefficient:",
			"T_ENGLISH": "English",
			"T_EXPERIENCE_ENERGY_CONSUMPTION": "Silver consumer cost:",
			"T_EYE": "eye",
			"T_FALLOW": "fallow",
			"T_FIRE": "fire",
			"T_FLOWER": "flower",
			"T_FOREST": "forest",
			"T_GENERAL": "General",
			"T_GENES": "Genes",
			"T_GENES2": "Genes 2",
			"T_GENES3": "Genes 3",
			"T_GERMAN": "German",
			"T_GOLD": "gold",
			"T_GRASS": "grass",
			"T_GRAY": "gray",
			"T_GREEN": "green",
			"T_HARDWARE_ACCELERATION": "Hardware acceleration",
			"T_HEALING": "Healing delay:",
			"T_HEIGHT": "Height:",
			"T_IMMUNE_SYSTEM": "Immune system delay:",
			"T_INDIGO": "indigo",
			"T_INDIGO_DIVISOR": "Indigo divisor:",
			"T_INITIAL_CARBON_DIOXIDE": "Initial carbon dioxide:",
			"T_INITIAL_COMPLEXITY": "Initial complexity:",
			"T_INITIAL_ENERGY": "Initial energy:",
			"T_INITIAL_METHANE": "Initial methane:",
			"T_INITIAL_NUMBER": "Initial number:",
			"T_INITIAL_OXYGEN": "Initial oxygen:",
			"T_JADE": "jade",
			"T_LANGUAGE": "Language:",
			"T_LAVENDER": "lavender",
			"T_LAVENDER_SHIELD": "Lavender shield:",
			"T_LEAF": "leaf",
			"T_LIFE_EXPECTANCY": "Life expectancy:",
			"T_LIFE_EXPECTANCY_DIVISOR": "Life expectancy divisor:",
			"T_LILAC": "lilac",
			"T_LIME": "lime",
			"T_MAGENTA": "magenta",
			"T_MAROON": "maroon",
			"T_MAX_MUTATION_PERCENTAGE": "Max mutation ‱:",
			"T_METABOLISM": "Metabolism",
			"T_META_MUTATION_PERCENTAGE": "Meta mutation ‱:",
			"T_MILLISECONDS": "milliseconds",
			"T_MINT": "mint",
			"T_MIN_MUTATION_PERCENTAGE": "Min mutation ‱:",
			"T_MOSQUITO_ENERGY_CONSUMPTION": "Spike consumer cost:",
			"T_NONE": "None",
			"T_OBTAINED_ENERGY": "Obtained energy:",
			"T_OBTAINED_ENERGY_DIVISOR": "Obtained energy divisor:",
			"T_OCHRE": "ochre",
			"T_OK": "OK",
			"T_OLIVE": "olive",
			"T_OPENGL": "OpenGL",
			"T_ORANGE": "orange",
			"T_ORGANISMS": "Organisms",
			"T_PARAMETERS_CONFIGURATION": "Parameters configuration",
			"T_PHOTOSYNTHETIC_METABOLISM": "Photosynthetic metabolism:",
			"T_PINK": "pink",
			"T_PLAGUE": "plague",
			"T_PORTUGUESE": "Portuguese",
			"T_PROBABILITY": "Probability:",
			"T_PURPLE": "purple",
			"T_RED": "red",
			"T_RELEASED_ENERGY_PROPORTION": "Released energy proportion:",
			"T_ROSE": "rose",
			"T_RUBBING_COEFFICIENT": "Rubbing coefficient:",
			"T_RUSSIAN": "Russian",
			"T_SCOURGE_ENERGY_CONSUMPTION": "Non-plant plague cost:",
			"T_SILVER": "silver",
			"T_SKY": "sky",
			"T_SPANISH": "Spanish",
			"T_SPIKE": "spike",
			"T_SPORE": "spore",
			"T_SPRING": "spring",
			"T_SUBSTANCE_DRAINAGE_DIVISOR": "Substances drainage divisor:",
			"T_SUMMER": "summer",
			"T_SYMBIONT_ENERGY_CONSUMPTION": "+ Photosynthetic effectivity with symbiont:",
			"T_TEAL": "teal",
			"T_TIME_BETWEEN_BACKUPS": "Time between backups:",
			"T_TIME_PER_FRAME": "Time per frame:",
			"T_UPKEEP_COST_DIVISOR": "Segment upkeep cost divisor:",
			"T_VIOLET": "violet",
			"T_VIRUS_ENERGY_CONSUMPTION": "Non-plant white cost:",
			"T_WHITE": "white",
			"T_WIDTH": "Width:",
			"T_WORLD": "World",
			"T_YELLOW": "yellow"
		},
		"es": {
			"T_AUBURN": "castaño",
			"T_AUTOMATIC_BACKUPS": "Respaldos automático",
			"T_BARK": "corteza",
			"T_BLOND": "rubio",
			"T_BLUE": "azul",
			"T_C4": "c4planta",
			"T_CANCEL": "Cancelar",
			"T_CATALAN": "Catalán",
			"T_CH4_TO_CO2_DIVISOR": "Divisor de CH4 a CO2:",
			"T_CHEMOORGANOTROPHIC_METABOLISM": "Metabolismo quimioorganotrófico:",
			"T_CLADE_COMPLEXITY": "Complejidad de un clado:",
			"T_CO2_TO_CH4_DIVISOR": "Divisor de CO2 a CH4:",
			"T_COLOR2": "Color",
			"T_CORAL": "coral",
			"T_COST": "Coste:",
			"T_CREAM": "crema",
			"T_CREAM_RELEASED_ENERGY_PROPORTION": "Crema proporción de energía desprendida:",
			"T_CROWDEDFOREST_ENERGY_CONSUMPTION": "+ Efectividad bosque si no solo:",
			"T_CROWDEDLIME_ENERGY_CONSUMPTION": "Efectividad lima si no solo:",
			"T_CYAN": "cian",
			"T_DARK": "negro",
			"T_DARKGRAY": "gris oscuro",
			"T_DARKJADE_DELAY": "Jade curación retraso:",
			"T_DECAY_ENERGY": "Energía perdida por los organismos muertos:",
			"T_DEFAULT_VALUES": "Valores por defecto",
			"T_DISABLE_FBOBJECT": "Deshabilitar fbobject (intentarlo si las OpenGL fallan)",
			"T_DODGE_ENERGY_CONSUMPTION": "Coste de esquivar:",
			"T_EFFECTIVITY": "Efectividad:",
			"T_ELASTICITY_COEFFICIENT": "Coeficiente de elasticidad:",
			"T_ENGLISH": "Inglés",
			"T_EXPERIENCE_ENERGY_CONSUMPTION": "Coste plata consumidor:",
			"T_EYE": "ojo",
			"T_FALLOW": "barbecho",
			"T_FIRE": "fuego",
			"T_FLOWER": "flor",
			"T_FOREST": "bosque",
			"T_GENERAL": "General",
			"T_GENES": "Genes",
			"T_GENES2": "Genes 2",
			"T_GENES3": "Genes 3",
			"T_GERMAN": "Alemán",
			"T_GOLD": "oro",
			"T_GRASS": "hierba",
			"T_GRAY": "gris",
			"T_GREEN": "verde",
			"T_HARDWARE_ACCELERATION": "Aceleración por hardware",
			"T_HEALING": "Curación retraso:",
			"T_HEIGHT": "Altura:",
			"T_IMMUNE_SYSTEM": "Sistema inmunitario retraso:",
			"T_INDIGO": "añil",
			"T_INDIGO_DIVISOR": "Divisor de añil:",
			"T_INITIAL_CARBON_DIOXIDE": "Dióxido de carbono inicial:",
			"T_INITIAL_COMPLEXITY": "Complejidad inicial:",
			"T_INITIAL_ENERGY": "Energía inicial:",
			"T_INITIAL_METHANE": "Metano inicial:",
			"T_INITIAL_NUMBER": "Número inicial:",
			"T_INITIAL_OXYGEN": "Oxígeno inicial:",
			"T_JADE": "jade",
			"T_LANGUAGE": "Idioma:",
			"T_LAVENDER": "lavanda",
			"T_LAVENDER_SHIELD": "Escudar lavanda:",
			"T_LEAF": "hoja",
			"T_LIFE_EXPECTANCY": "Esperanza de vida:",
			"T_LIFE_EXPECTANCY_DIVISOR": "Divisor de tiempo de vida:",
			"T_LILAC": "lila",
			"T_LIME": "lima",
			"T_MAGENTA": "magenta",
			"T_MAROON": "granate",
			"T_MAX_MUTATION_PERCENTAGE": "‱ de mutación max:",
			"T_METABOLISM": "Metabolismo",
			"T_META_MUTATION_PERCENTAGE": "‱ de mutación meta:",
			"T_MILLISECONDS": "milisegundos",
			"T_MINT": "menta",
			"T_MIN_MUTATION_PERCENTAGE": "‱ de mutación min:",
			"T_MOSQUITO_ENERGY_CONSUMPTION": "Coste espiga consumidor:",
			"T_NONE": "Ninguna",
			"T_OBTAINED_ENERGY": "Energía obtenida:",
			"T_OBTAINED_ENERGY_DIVISOR": "Divisor de la energía obtenida:",
			"T_OCHRE": "ocre",
			"T_OK": "Aceptar",
			"T_OLIVE": "oliva",
			"T_OPENGL": "OpenGL",
			"T_ORANGE": "naranja",
			"T_ORGANISMS": "Organismos",
			"T_PARAMETERS_CONFIGURATION": "Configuración de los parámetros",
			"T_PHOTOSYNTHETIC_METABOLISM": "Metabolismo fotosintético:",
			"T_PINK": "rosa",
			"T_PLAGUE": "plaga",
			"T_PORTUGUESE": "Portugués",
			"T_PROBABILITY": "Probabilidad:",
			"T_PURPLE": "púrpura",
			"T_RED": "rojo",
			"T_RELEASED_ENERGY_PROPORTION": "Proporción de energía desprendida:",
			"T_ROSE": "fucsia",
			"T_RUBBING_COEFFICIENT": "Coeficiente de rozamiento:",
			"T_RUSSIAN": "Ruso",
			"T_SCOURGE_ENERGY_CONSUMPTION": "Coste peste no vegetal:",
			"T_SILVER": "plata",
			"T_SKY": "azur",
			"T_SPANISH": "Español",
			"T_SPIKE": "espiga",
			"T_SPORE": "espora",
			"T_SPRING": "primavera",
			"T_SUBSTANCE_DRAINAGE_DIVISOR": "Divisor al drenage de sustancias:",
			"T_SUMMER": "verano",
			"T_SYMBIONT_ENERGY_CONSUMPTION": "+ Efectividad fotosíntesis con simbionte:",
			"T_TEAL": "teal",
			"T_TIME_BETWEEN_BACKUPS": "Tiempo entre respaldos",
			"T_TIME_PER_FRAME": "Tiempo por imagen:",
			"T_UPKEEP_COST_DIVISOR": "Divisor para el coste de mantenimiento de los segmentos:",
			"T_VIOLET": "violeta",
			"T_VIRUS_ENERGY_CONSUMPTION": "Coste blanco no vegetal:",
			"T_WHITE": "blanco",
			"T_WIDTH": "Anchura:",
			"T_WORLD": "Mundo",
			"T_YELLOW": "amarillo"
		},
		"ca": {
			"T_AUBURN": "castanyer",
			"T_AUTOMATIC_BACKUPS": "Cópies de seguretat automàtic",
			"T_BARK": "escorça",
			"T_BLOND": "ros",
			"T_BLUE": "blau",
			"T_C4": "c4planta",
			"T_CANCEL": "Cancel·la",
			"T_CATALAN": "Català",
			"T_CH4_TO_CO2_DIVISOR": "Divisor de CH4 a CO2:",
			"T_CHEMOORGANOTROPHIC_METABOLISM": "Metabolisme quimioorganotròfic:",
			"T_CLADE_COMPLEXITY": "Complexitat d'un clado:",
			"T_CO2_TO_CH4_DIVISOR": "Divisor de CO2 a CH4:",
			"T_COLOR2": "Color",
			"T_CORAL": "corall",
			"T_COST": "Cost:",
			"T_CREAM": "crema",
			"T_CREAM_RELEASED_ENERGY_PROPORTION": "Crema proporció d'energia despresa:",
			"T_CROWDEDFOREST_ENERGY_CONSUMPTION": "+ Efectivitat bosc si no només:",
			"T_CROWDEDLIME_ENERGY_CONSUMPTION": "Efectivitat llima si no només:",
			"T_CYAN": "cian",
			"T_DARK": "negre",
			"T_DARKGRAY": "gris fosc",
			"T_DARKJADE_DELAY": "Jade guariment retard:",
			"T_DECAY_ENERGY": "Energia perduda pels organismes morts:",
			"T_DEFAULT_VALUES": "Valors per defecte",
			"T_DISABLE_FBOBJECT": "Deshabilita fbobject (intenteu-ho si fallen les OpenGL)",
			"T_DODGE_ENERGY_CONSUMPTION": "Cost d'esquivar:",
			"T_EFFECTIVITY": "Efectivitat:",
			"T_ELASTICITY_COEFFICIENT": "Coeficient d'elasticitat:",
			"T_ENGLISH": "Anglès",
			"T_EXPERIENCE_ENERGY_CONSUMPTION": "Cost plata consumir:",
			"T_EYE": "ull",
			"T_FALLOW": "guaret",
			"T_FIRE": "foc",
			"T_FLOWER": "flor",
			"T_FOREST": "bosc",
			"T_GENERAL": "General",
			"T_GENES": "Gens",
			"T_GENES2": "Gens 2",
			"T_GENES3": "Gens 3",
			"T_GERMAN": "Alemany",
			"T_GOLD": "or",
			"T_GRASS": "herba",
			"T_GRAY": "gris",
			"T_GREEN": "verd",
			"T_HARDWARE_ACCELERATION": "Acceleració per maquinari",
			"T_HEALING": "Guariment retard:",
			"T_HEIGHT": "Alçada:",
			"T_IMMUNE_SYSTEM": "Sistema immunitari retard:",
			"T_INDIGO": "anyil",
			"T_INDIGO_DIVISOR": "Divisor de anyil:",
			"T_INITIAL_CARBON_DIOXIDE": "Diòxid de carboni inicial:",
			"T_INITIAL_COMPLEXITY": "Complexitat inicial:",
			"T_INITIAL_ENERGY": "Energia inicial:",
			"T_INITIAL_METHANE": "Metà inicial:",
			"T_INITIAL_NUMBER": "Nombre inicial:",
			"T_INITIAL_OXYGEN": "Oxigen inicial:",
			"T_JADE": "jade",
			"T_LANGUAGE": "Idioma:",
			"T_LAVENDER": "lavanda",
			"T_LAVENDER_SHIELD": "Escut lavanda:",
			"T_LEAF": "fulla",
			"T_LIFE_EXPECTANCY": "Esperança de vida:",
			"T_LIFE_EXPECTANCY_DIVISOR": "Divisor de temps de vida:",
			"T_LILAC": "lilà",
			"T_LIME": "llima",
			"T_MAGENTA": "magenta",
			"T_MAROON": "granat",
			"T_MAX_MUTATION_PERCENTAGE": "‱ de mutació màx:",
			"T_METABOLISM": "Metabolisme",
			"T_META_MUTATION_PERCENTAGE": "‱ de mutació meta:",
			"T_MILLISECONDS": "mil·lisegons",
			"T_MINT": "menta",
			"T_MIN_MUTATION_PERCENTAGE": "‱ de mutació min:",
			"T_MOSQUITO_ENERGY_CONSUMPTION": "Cost espiga consumir:",
			"T_NONE": "Cap",
			"T_OBTAINED_ENERGY": "Energia obtinguda:",
			"T_OBTAINED_ENERGY_DIVISOR": "Divisor de l'energia obtinguda:",
			"T_OCHRE": "ocre",
			"T_OK": "Accepta",
			"T_OLIVE": "oliva",
			"T_OPENGL": "OpenGL",
			"T_ORANGE": "taronja",
			"T_ORGANISMS": "Organismes",
			"T_PARAMETERS_CONFIGURATION": "Configuració dels paràmetres",
			"T_PHOTOSYNTHETIC_METABOLISM": "Metabolisme fotosintètic:",
			"T_PINK": "rosa",
			"T_PLAGUE": "plaga",
			"T_PORTUGUESE": "Portuguès",
			"T_PROBABILITY": "Probabilitat:",
			"T_PURPLE": "porpra",
			"T_RED": "vermell",
			"T_RELEASED_ENERGY_PROPORTION": "Proporció d'energia despresa:",
			"T_ROSE": "fúcsia",
			"T_RUBBING_COEFFICIENT": "Coeficient de fregament:",
			"T_RUSSIAN": "Rus",
			"T_SCOURGE_ENERGY_CONSUMPTION": "Cost pesta no vegetal:",
			"T_SILVER": "plata",
			"T_SKY": "atzur",
			"T_SPANISH": "Castellà",
			"T_SPIKE": "espiga",
			"T_SPORE": "espora",
			"T_SPRING": "primavera",
			"T_SUBSTANCE_DRAINAGE_DIVISOR": "Divisor al drenatge de substàncies:",
			"T_SUMMER": "estiu",
			"T_SYMBIONT_ENERGY_CONSUMPTION": "+ Efectivitat fotosíntesi amb symbiont:",
			"T_TEAL": "xarxet",
			"T_TIME_BETWEEN_BACKUPS": "Temps entre cópies de seguretat:",
			"T_TIME_PER_FRAME": "Temps per imatge:",
			"T_UPKEEP_COST_DIVISOR": "Divisor pel cost de manteniment dels segments:",
			"T_VIOLET": "violat",
			"T_VIRUS_ENERGY_CONSUMPTION": "Cost blanc no vegetal:",
			"T_WHITE": "blanc",
			"T_WIDTH": "Amplada:",
			"T_WORLD": "Món",
			"T_YELLOW": "groc"
		},
		"de": {
			"T_AUBURN": "rotbraun",
			"T_AUTOMATIC_BACKUPS": "Automatische Sicherungskopien",
			"T_BARK": "rinde",
			"T_BLOND": "blond",
			"T_BLUE": "blau",
			"T_C4": "c4pflanze",
			"T_CANCEL": "Abbrechen",
			"T_CATALAN": "Katalanisch",
			"T_CH4_TO_CO2_DIVISOR": "CH4 zu CO2 Divisor:",
			"T_CHEMOORGANOTROPHIC_METABOLISM": "Chemoorganotrophischer Stoffwechsel:",
			"T_CLADE_COMPLEXITY": "Komplexität einer Klade:",
			"T_CO2_TO_CH4_DIVISOR": "CO2 zu CH4 Divisor:",
			"T_COLOR2": "Farbe",
			"T_CORAL": "koralle",
			"T_COST": "Kosten:",
			"T_CREAM": "creme",
			"T_CREAM_RELEASED_ENERGY_PROPORTION": "Creme Anteil der freigesetzten Energie:",
			"T_CROWDEDFOREST_ENERGY_CONSUMPTION": "+ Waldgrün-Effektivität wenn nicht allein:",
			"T_CROWDEDLIME_ENERGY_CONSUMPTION": "Lindgrün-Effektivität wenn nicht allein:",
			"T_CYAN": "cyan",
			"T_DARK": "schwarz",
			"T_DARKGRAY": "dunkelgrau",
			"T_DARKJADE_DELAY": "Jade Heilung Verzögerung:",
			"T_DECAY_ENERGY": "Energieverlust toter Organismen:",
			"T_DEFAULT_VALUES": "Standard-Werte",
			"T_DISABLE_FBOBJECT": "Deaktiviere fbobject (versuche dies falls OpenGL nicht funktioniert)",
			"T_DODGE_ENERGY_CONSUMPTION": "Kosten des Ausweichens:",
			"T_EFFECTIVITY": "Effektivität:",
			"T_ELASTICITY_COEFFICIENT": "Elastizitätskoeffizient:",
			"T_ENGLISH": "Englisch",
			"T_EXPERIENCE_ENERGY_CONSUMPTION": "Silber-Kosten für Konsumenten:",
			"T_EYE": "auge",
			"T_FALLOW": "falb",
			"T_FIRE": "feuer",
			"T_FLOWER": "blüte",
			"T_FOREST": "waldgrün",
			"T_GENERAL": "Allgemein",
			"T_GENES": "Gene",
			"T_GENES2": "Gene 2",
			"T_GENES3": "Gene 3",
			"T_GERMAN": "Deutsch",
			"T_GOLD": "gold",
			"T_GRASS": "gras",
			"T_GRAY": "grau",
			"T_GREEN": "grün",
			"T_HARDWARE_ACCELERATION": "Hardwarebeschleunigung",
			"T_HEALING": "Heilung Verzögerung:",
			"T_HEIGHT": "Höhe:",
			"T_IMMUNE_SYSTEM": "Immunsystem Verzögerung:",
			"T_INDIGO": "indigo",
			"T_INDIGO_DIVISOR": "Indigo Divisor:",
			"T_INITIAL_CARBON_DIOXIDE": "Kohlendioxid zu Beginn:",
			"T_INITIAL_COMPLEXITY": "Komplexität zu Beginn:",
			"T_INITIAL_ENERGY": "Energie zu Beginn:",
			"T_INITIAL_METHANE": "Methan zu Beginn:",
			"T_INITIAL_NUMBER": "Anzahl zu Beginn:",
			"T_INITIAL_OXYGEN": "Sauerstoff zu Beginn:",
			"T_JADE": "jade",
			"T_LANGUAGE": "Sprache:",
			"T_LAVENDER": "lavendel",
			"T_LAVENDER_SHIELD": "Lavendel Schild:",
			"T_LEAF": "blatt",
			"T_LIFE_EXPECTANCY": "Lebenserwartung:",
			"T_LIFE_EXPECTANCY_DIVISOR": "Lebenserwartung Divisor:",
			"T_LILAC": "lila",
			"T_LIME": "lindgrün",
			"T_MAGENTA": "magenta",
			"T_MAROON": "weinrot",
			"T_MAX_MUTATION_PERCENTAGE": "Max Mutation ‱:",
			"T_METABOLISM": "Stoffwechsel",
			"T_META_MUTATION_PERCENTAGE": "Meta Mutation ‱:",
			"T_MILLISECONDS": "Millisekunden",
			"T_MINT": "minze",
			"T_MIN_MUTATION_PERCENTAGE": "Min Mutation ‱:",
			"T_MOSQUITO_ENERGY_CONSUMPTION": "Stachel-Kosten für Konsumenten:",
			"T_NONE": "Keine",
			"T_OBTAINED_ENERGY": "Entnommene Energie:",
			"T_OBTAINED_ENERGY_DIVISOR": "Gewonnene Energie Divisor:",
			"T_OCHRE": "ocker",
			"T_OK": "OK",
			"T_OLIVE": "oliv",
			"T_OPENGL": "OpenGL",
			"T_ORANGE": "orange",
			"T_ORGANISMS": "Organismen",
			"T_PARAMETERS_CONFIGURATION": "Parametereinstellung",
			"T_PHOTOSYNTHETIC_METABOLISM": "Photosynthetischer Stoffwechsel:",
			"T_PINK": "pink",
			"T_PLAGUE": "plage",
			"T_PORTUGUESE": "Portugiesisch",
			"T_PROBABILITY": "Wahrscheinlichkeit:",
			"T_PURPLE": "purpur",
			"T_RED": "rot",
			"T_RELEASED_ENERGY_PROPORTION": "Anteil der freigesetzten Energie:",
			"T_ROSE": "rosa",
			"T_RUBBING_COEFFICIENT": "Reibungskoeffizient:",
			"T_RUSSIAN": "Russisch",
			"T_SCOURGE_ENERGY_CONSUMPTION": "Seuche-Kosten für Nichtpflanzen:",
			"T_SILVER": "silber",
			"T_SKY": "azur",
			"T_SPANISH": "Spanisch",
			"T_SPIKE": "stachel",
			"T_SPORE": "spore",
			"T_SPRING": "hellgrün",
			"T_SUBSTANCE_DRAINAGE_DIVISOR": "Substanzenabfluss Divisor:",
			"T_SUMMER": "sommergrün",
			"T_SYMBIONT_ENERGY_CONSUMPTION": "+ Photosynthese-Effektivität mit Symbiont:",
			"T_TEAL": "türkis",
			"T_TIME_BETWEEN_BACKUPS": "Zeit zwischen Sicherungskopien:",
			"T_TIME_PER_FRAME": "Zeit zwischen Frames:",
			"T_UPKEEP_COST_DIVISOR": "Segmentunterhaltskosten Divisor:",
			"T_VIOLET": "violett",
			"T_VIRUS_ENERGY_CONSUMPTION": "Weiß-Kosten für Nichtpflanzen:",
			"T_WHITE": "weiß",
			"T_WIDTH": "Breite:",
			"T_WORLD": "Welt",
			"T_YELLOW": "gelb"
		},
		"pt": {
			"T_AUBURN": "ruivo",
			"T_AUTOMATIC_BACKUPS": "Cópias de segurança automáticas",
			"T_BARK": "súber",
			"T_BLOND": "loiro",
			"T_BLUE": "azul",
			"T_C4": "c4plantar",
			"T_CANCEL": "Cancelar",
			"T_CATALAN": "Catalão",
			"T_CH4_TO_CO2_DIVISOR": "Divisor de CH4 para CO2:",
			"T_CHEMOORGANOTROPHIC_METABOLISM": "Metabolismo quimiorganotrófico:",
			"T_CLADE_COMPLEXITY": "Complexidade de um clado:",
			"T_CO2_TO_CH4_DIVISOR": "Divisor de CO2 para CH4:",
			"T_COLOR2": "Cor",
			"T_CORAL": "coral",
			"T_COST": "Custo:",
			"T_CREAM": "creme",
			"T_CREAM_RELEASED_ENERGY_PROPORTION": "Creme proporção de energia libertada:",
			"T_CROWDEDFOREST_ENERGY_CONSUMPTION": "+ Efetividade floresta se não sozinha:",
			"T_CROWDEDLIME_ENERGY_CONSUMPTION": "Efetividade lima se não sozinha:",
			"T_CYAN": "ciano",
			"T_DARK": "preto",
			"T_DARKGRAY": "cinza-escuro",
			"T_DARKJADE_DELAY": "Atraso da cura jade:",
			"T_DECAY_ENERGY": "Energia perdida pela morte dos organismos:",
			"T_DEFAULT_VALUES": "Valores padrão",
			"T_DISABLE_FBOBJECT": "Desativar fbobject (tente se o OpenGL não funciona)",
			"T_DODGE_ENERGY_CONSUMPTION": "Custo de esquivando-se:",
			"T_EFFECTIVITY": "Efetividade:",
			"T_ELASTICITY_COEFFICIENT": "Coeficiente de elasticidade:",
			"T_ENGLISH": "Inglês",
			"T_EXPERIENCE_ENERGY_CONSUMPTION": "Custo prata consumir:",
			"T_EYE": "olho",
			"T_FALLOW": "pousio",
			"T_FIRE": "fogo",
			"T_FLOWER": "flor",
			"T_FOREST": "floresta",
			"T_GENERAL": "Geral",
			"T_GENES": "Genes",
			"T_GENES2": "Genes 2",
			"T_GENES3": "Genes 3",
			"T_GERMAN": "Alemão",
			"T_GOLD": "dourado",
			"T_GRASS": "erva",
			"T_GRAY": "cinza",
			"T_GREEN": "verde",
			"T_HARDWARE_ACCELERATION": "Aceleração de hardware",
			"T_HEALING": "Atraso da cura:",
			"T_HEIGHT": "Altura:",
			"T_IMMUNE_SYSTEM": "Atraso do sistema imunitário:",
			"T_INDIGO": "anil",
			"T_INDIGO_DIVISOR": "Divisor de anil:",
			"T_INITIAL_CARBON_DIOXIDE": "Dióxido de carbono inicial:",
			"T_INITIAL_COMPLEXITY": "Complexidade inicial:",
			"T_INITIAL_ENERGY": "Energia inicial:",
			"T_INITIAL_METHANE": "Metano inicial:",
			"T_INITIAL_NUMBER": "Número inicial:",
			"T_INITIAL_OXYGEN": "Oxigênio inicial:",
			"T_JADE": "jade",
			"T_LANGUAGE": "Linguagem:",
			"T_LAVENDER": "lavanda",
			"T_LAVENDER_SHIELD": "Escudo lavanda:",
			"T_LEAF": "folha",
			"T_LIFE_EXPECTANCY": "Expectativa de vida:",
			"T_LIFE_EXPECTANCY_DIVISOR": "Divisor de expectativa de vida:",
			"T_LILAC": "lilás",
			"T_LIME": "lima",
			"T_MAGENTA": "magenta",
			"T_MAROON": "bordô",
			"T_MAX_MUTATION_PERCENTAGE": "Mutação máxima ‱:",
			"T_METABOLISM": "Metabolismo",
			"T_META_MUTATION_PERCENTAGE": "Mutação meta ‱:",
			"T_MILLISECONDS": "milissegundos",
			"T_MINT": "menta",
			"T_MIN_MUTATION_PERCENTAGE": "Mutação mínima ‱:",
			"T_MOSQUITO_ENERGY_CONSUMPTION": "Custo espiga consumir:",
			"T_NONE": "Nenhuma",
			"T_OBTAINED_ENERGY": "Energia obtida:",
			"T_OBTAINED_ENERGY_DIVISOR": "Divisor de energia obtida:",
			"T_OCHRE": "ocre",
			"T_OK": "OK",
			"T_OLIVE": "verde-oliva",
			"T_OPENGL": "OpenGL",
			"T_ORANGE": "laranja",
			"T_ORGANISMS": "Organismos",
			"T_PARAMETERS_CONFIGURATION": "Configuração dos parâmetros",
			"T_PHOTOSYNTHETIC_METABOLISM": "Metabolismo foto-sintético:",
			"T_PINK": "rosa",
			"T_PLAGUE": "praga",
			"T_PORTUGUESE": "Português",
			"T_PROBABILITY": "Probabilidade:",
			"T_PURPLE": "púrpura",
			"T_RED": "vermelho",
			"T_RELEASED_ENERGY_PROPORTION": "Proporção de energia libertada:",
			"T_ROSE": "fúchsia",
			"T_RUBBING_COEFFICIENT": "Coeficiente de atrito:",
			"T_RUSSIAN": "Russo",
			"T_SCOURGE_ENERGY_CONSUMPTION": "Custo peste não vegetal:",
			"T_SILVER": "prata",
			"T_SKY": "céu",
			"T_SPANISH": "Espanhol",
			"T_SPIKE": "espiga",
			"T_SPORE": "esporo",
			"T_SPRING": "primavera",
			"T_SUBSTANCE_DRAINAGE_DIVISOR": "Divisor de drenagem de substâncias:",
			"T_SUMMER": "verão",
			"T_SYMBIONT_ENERGY_CONSUMPTION": "+ Efetividade fotossíntese com simbionte:",
			"T_TEAL": "turquesa",
			"T_TIME_BETWEEN_BACKUPS": "Tempo entre cópias de segurança:",
			"T_TIME_PER_FRAME": "Tempo por fotograma:",
			"T_UPKEEP_COST_DIVISOR": "Divisor de custo para manutenção do segmento:",
			"T_VIOLET": "violeta",
			"T_VIRUS_ENERGY_CONSUMPTION": "Custo branco não vegetal:",
			"T_WHITE": "branco",
			"T_WIDTH": "Largura:",
			"T_WORLD": "Ecossistema",
			"T_YELLOW": "amarelo"
		},
		"ru": {
			"T_AUBURN": "рыжеватый",
			"T_AUTOMATIC_BACKUPS": "Автоматическое резервное копирование",
			"T_BARK": "древесный",
			"T_BLOND": "блонд",
			"T_BLUE": "синий",
			"T_C4": "растительный",
			"T_CANCEL": "Отмена",
			"T_CATALAN": "Каталанский",
			"T_CH4_TO_CO2_DIVISOR": "Делитель CH4 на CO2",
			"T_CHEMOORGANOTROPHIC_METABOLISM": "Хемоорганотрофический метаболизм:",
			"T_CLADE_COMPLEXITY": "Сложность клэда:",
			"T_CO2_TO_CH4_DIVISOR": "Делитель CO2 на CH4:",
			"T_COLOR2": "Цвет",
			"T_CORAL": "коралловый",
			"T_COST": "Затраты:",
			"T_CREAM": "кремовый",
			"T_CREAM_RELEASED_ENERGY_PROPORTION": "Пропорция освобождённой энергии сливок:",
			"T_CROWDEDFOREST_ENERGY_CONSUMPTION": "+ Эффективность леса, если не одна:",
			"T_CROWDEDLIME_ENERGY_CONSUMPTION": "Эффективность лайма, если не одна:",
			"T_CYAN": "голубой",
			"T_DARK": "чёрный",
			"T_DARKGRAY": "тёмно-серый",
			"T_DARKJADE_DELAY": "Задержка лечения нефрита:",
			"T_DECAY_ENERGY": "Потери энергии мёртвыми организмами:",
			"T_DEFAULT_VALUES": "Значения по умолчанию",
			"T_DISABLE_FBOBJECT": "Отключить fbobject (попытаться, если OpenGL не работает)",
			"T_DODGE_ENERGY_CONSUMPTION": "Затраты на уклонения:",
			"T_EFFECTIVITY": "Экономичность:",
			"T_ELASTICITY_COEFFICIENT": "Коэффициент эластичности:",
			"T_ENGLISH": "Английский",
			"T_EXPERIENCE_ENERGY_CONSUMPTION": "Затраты на серебро:",
			"T_EYE": "глазной",
			"T_FALLOW": "осенний",
			"T_FIRE": "огненный",
			"T_FLOWER": "цветочный",
			"T_FOREST": "лесной",
			"T_GENERAL": "Общее",
			"T_GENES": "Гены",
			"T_GENES2": "Гены 2",
			"T_GENES3": "Гены 3",
			"T_GERMAN": "Немецкий",
			"T_GOLD": "золотой",
			"T_GRASS": "травяной",
			"T_GRAY": "серый",
			"T_GREEN": "зелёный",
			"T_HARDWARE_ACCELERATION": "Аппаратное ускорение",
			"T_HEALING": "Задержка лечения:",
			"T_HEIGHT": "Высота:",
			"T_IMMUNE_SYSTEM": "Задержка иммунной системы:",
			"T_INDIGO": "индиго",
			"T_INDIGO_DIVISOR": "Делитель индиго:",
			"T_INITIAL_CARBON_DIOXIDE": "Первоначальный углекислый газ:",
			"T_INITIAL_COMPLEXITY": "Первоначальная сложность:",
			"T_INITIAL_ENERGY": "Первоначальная энергия:",
			"T_INITIAL_METHANE": "Первоначальный метан:",
			"T_INITIAL_NUMBER": "Первоначальное число:",
			"T_INITIAL_OXYGEN": "Первоначальный кислород:",
			"T_JADE": "нефритовый",
			"T_LANGUAGE": "Язык:",
			"T_LAVENDER": "лавандовый",
			"T_LAVENDER_SHIELD": "Знак лаванды:",
			"T_LEAF": "лиственный",
			"T_LIFE_EXPECTANCY": "Макс. возраст:",
			"T_LIFE_EXPECTANCY_DIVISOR": "Делитель макс. возраста:",
			"T_LILAC": "сиреневый",
			"T_LIME": "лайм",
			"T_MAGENTA": "пурпурный",
			"T_MAROON": "каштановый",
			"T_MAX_MUTATION_PERCENTAGE": "Макс. мутация ‱:",
			"T_METABOLISM": "Метаболизм",
			"T_META_MUTATION_PERCENTAGE": "Метамутация ‱:",
			"T_MILLISECONDS": "мсек",
			"T_MINT": "мятный",
			"T_MIN_MUTATION_PERCENTAGE": "Мин. мутация ‱:",
			"T_MOSQUITO_ENERGY_CONSUMPTION": "Затраты на колючку:",
			"T_NONE": "Отсутствует",
			"T_OBTAINED_ENERGY": "Освобождённая энергия:",
			"T_OBTAINED_ENERGY_DIVISOR": "Делитель полученной энергии:",
			"T_OCHRE": "охристый",
			"T_OK": "ОК",
			"T_OLIVE": "оливковый",
			"T_OPENGL": "OpenGL",
			"T_ORANGE": "оранжевый",
			"T_ORGANISMS": "Организмы",
			"T_PARAMETERS_CONFIGURATION": "Параметры конфигурации",
			"T_PHOTOSYNTHETIC_METABOLISM": "Фотосинтетический метаболизм:",
			"T_PINK": "розовый",
			"T_PLAGUE": "чумной",
			"T_PORTUGUESE": "Португальский",
			"T_PROBABILITY": "Вероятность:",
			"T_PURPLE": "пурпурный",
			"T_RED": "красный",
			"T_RELEASED_ENERGY_PROPORTION": "Пропорция освобождённой энергии:",
			"T_ROSE": "палевый",
			"T_RUBBING_COEFFICIENT": "Коэффициент трения:",
			"T_RUSSIAN": "Русский",
			"T_SCOURGE_ENERGY_CONSUMPTION": "Затраты на чуму для не-растений:",
			"T_SILVER": "серебряный",
			"T_SKY": "небесный",
			"T_SPANISH": "Испанский",
			"T_SPIKE": "колючий",
			"T_SPORE": "спор",
			"T_SPRING": "весенний",
			"T_SUBSTANCE_DRAINAGE_DIVISOR": "Делитель дренажа субстанций:",
			"T_SUMMER": "летний",
			"T_SYMBIONT_ENERGY_CONSUMPTION": "+ Эффективность фотосинтеза для симбионта:",
			"T_TEAL": "чирок",
			"T_TIME_BETWEEN_BACKUPS": "Время между резервными копированиями:",
			"T_TIME_PER_FRAME": "Время на кадр:",
			"T_UPKEEP_COST_DIVISOR": "Делитель затрат на ремонт сегмента:",
			"T_VIOLET": "фиолетовый",
			"T_VIRUS_ENERGY_CONSUMPTION": "Белые затраты для не-растений:",
			"T_WHITE": "белый",
			"T_WIDTH": "Ширина:",
			"T_WORLD": "Мир",
			"T_YELLOW": "жёлтый"
		}
	},
	"languages": [
		"T_ENGLISH",
		"T_SPANISH",
		"T_CATALAN",
		"T_GERMAN",
		"T_PORTUGUESE",
		"T_RUSSIAN"
	],
	"sections": [
		{
			"tab": "Metabolism",
			"before_key": "GREEN_OBTAINED_ENERGY_DIVISOR",
			"label_key": "T_PHOTOSYNTHETIC_METABOLISM"
		},
		{
			"tab": "Metabolism",
			"before_key": "ORGANIC_OBTAINED_ENERGY",
			"label_key": "T_CHEMOORGANOTROPHIC_METABOLISM"
		}
	]
}
