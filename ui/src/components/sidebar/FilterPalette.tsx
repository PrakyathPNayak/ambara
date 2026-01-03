import { useState, useMemo } from 'react';
import { FilterInfo, FilterCategory } from '../../types';
import './FilterPalette.css';

interface FilterPaletteProps {
  filters: FilterInfo[];
  onAddFilter: (filter: FilterInfo) => void;
}

// Category display order matching Rust Category::all()
const categoryOrder: FilterCategory[] = [
  'Input',
  'Output',
  'Transform',
  'Adjust',
  'Blur',
  'Sharpen',
  'Edge',
  'Noise',
  'Draw',
  'Text',
  'Composite',
  'Color',
  'Analyze',
  'Math',
  'Utility',
  'Custom',
];

export function FilterPalette({ filters, onAddFilter }: FilterPaletteProps) {
  const [searchQuery, setSearchQuery] = useState('');
  const [expandedCategories, setExpandedCategories] = useState<Set<string>>(
    new Set(categoryOrder)
  );

  const filteredFilters = useMemo(() => {
    if (!searchQuery.trim()) return filters;
    const query = searchQuery.toLowerCase();
    return filters.filter(
      (f) =>
        f.name.toLowerCase().includes(query) ||
        f.description.toLowerCase().includes(query) ||
        f.category.toLowerCase().includes(query)
    );
  }, [filters, searchQuery]);

  const groupedFilters = useMemo(() => {
    const groups: Partial<Record<FilterCategory, FilterInfo[]>> = {};

    // Initialize all categories
    categoryOrder.forEach((cat) => {
      groups[cat] = [];
    });

    filteredFilters.forEach((filter) => {
      const category = filter.category as FilterCategory;
      if (!groups[category]) {
        groups[category] = [];
      }
      groups[category]!.push(filter);
    });

    return groups;
  }, [filteredFilters]);

  const toggleCategory = (category: string) => {
    setExpandedCategories((prev) => {
      const next = new Set(prev);
      if (next.has(category)) {
        next.delete(category);
      } else {
        next.add(category);
      }
      return next;
    });
  };

  return (
    <div className="filter-palette">
      <div className="filter-palette-header">
        <h3>Filters</h3>
        <input
          type="text"
          className="filter-search"
          placeholder="Search filters..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
        />
      </div>

      <div className="filter-categories">
        {categoryOrder.map((category) => {
          const categoryFilters = groupedFilters[category] || [];
          if (categoryFilters.length === 0) return null;

          const isExpanded = expandedCategories.has(category);

          return (
            <div key={category} className="filter-category">
              <button
                className="category-header"
                onClick={() => toggleCategory(category)}
              >
                <span className="category-toggle">{isExpanded ? '▼' : '▶'}</span>
                <span className="category-name">{category}</span>
                <span className="category-count">{categoryFilters.length}</span>
              </button>

              {isExpanded && (
                <div className="category-filters">
                  {categoryFilters.map((filter) => (
                    <button
                      key={filter.id}
                      className="filter-item"
                      onClick={() => onAddFilter(filter)}
                      title={filter.description}
                    >
                      <span className="filter-name">{filter.name}</span>
                      <span className="filter-ports">
                        {filter.inputs.length}→{filter.outputs.length}
                      </span>
                    </button>
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
