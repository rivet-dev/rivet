// @ts-nocheck
/* eslint-disable */
'use client';
import { defaultEvents, NullState, useIconSelection, } from './chunk-K6MKSK76.js';
import { className, className2, css, css2, fonts, fonts2, } from './chunk-76DSJYNX.js';
import './chunk-RUAN5HWR.js';

// https :https://framerusercontent.com/modules/EQc0WVLWSNg3MefFYC3W/q1ZVKhr4UZwXUATr4xTE/kTXE7wUBN.js
import { jsx as _jsx5, jsxs as _jsxs4, } from 'react/jsx-runtime';
import {
  addFonts as addFonts4,
  addPropertyControls as addPropertyControls5,
  ComponentViewportProvider,
  ControlType as ControlType5,
  cx as cx4,
  getFonts,
  getFontsFromSharedStyle,
  RichText as RichText4,
  SVG as SVG3,
  useComponentViewport as useComponentViewport4,
  useLocaleInfo as useLocaleInfo4,
  useVariantState as useVariantState4,
  withCSS as withCSS4,
} from 'unframer';
import { LayoutGroup as LayoutGroup4, motion as motion4, MotionConfigContext as MotionConfigContext4, } from 'unframer';
import * as React5 from 'react';

// https :https://framerusercontent.com/modules/f0DboytQenYh21kfme7W/zb1zVBMZJKgPMiedOi0y/Feather.js
import { jsx as _jsx, } from 'react/jsx-runtime';
import * as React from 'react';
import { useEffect, useRef, useState, } from 'react';
import { addPropertyControls, ControlType, RenderTarget, } from 'unframer';

// https :https://framer.com/m/feather-icons/home.js@0.0.29
var r;
var s = (o,) => {
  if (!r) {
    const n = o.forwardRef(({ color: t = 'currentColor', size: e = 24, ...i }, l,) =>
      o.createElement(
        'svg',
        {
          ref: l,
          xmlns: 'http://www.w3.org/2000/svg',
          width: e,
          height: e,
          viewBox: '0 0 24 24',
          fill: 'none',
          stroke: t,
          strokeWidth: '2',
          strokeLinecap: 'round',
          strokeLinejoin: 'round',
          ...i,
        },
        o.createElement('path', { d: 'M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z', },),
        o.createElement('polyline', { points: '9 22 9 12 15 12 15 22', },),
      )
    );
    n.displayName = 'Home', r = n;
  }
  return r;
};

// https :https://framerusercontent.com/modules/f0DboytQenYh21kfme7W/zb1zVBMZJKgPMiedOi0y/Feather.js
var iconKeys = [
  'activity',
  'airplay',
  'alert-circle',
  'alert-octagon',
  'alert-triangle',
  'align-center',
  'align-justify',
  'align-left',
  'align-right',
  'anchor',
  'aperture',
  'archive',
  'arrow-down',
  'arrow-down-circle',
  'arrow-down-left',
  'arrow-down-right',
  'arrow-left',
  'arrow-left-circle',
  'arrow-right',
  'arrow-right-circle',
  'arrow-up',
  'arrow-up-circle',
  'arrow-up-left',
  'arrow-up-right',
  'at-sign',
  'award',
  'bar-chart',
  'bar-chart-2',
  'battery',
  'battery-charging',
  'bell',
  'bell-off',
  'bluetooth',
  'bold',
  'book',
  'book-open',
  'bookmark',
  'box',
  'briefcase',
  'calendar',
  'camera',
  'camera-off',
  'cast',
  'check',
  'check-circle',
  'check-square',
  'chevron-down',
  'chevron-left',
  'chevron-right',
  'chevron-up',
  'chevrons-down',
  'chevrons-left',
  'chevrons-right',
  'chevrons-up',
  'chrome',
  'circle',
  'clipboard',
  'clock',
  'cloud',
  'cloud-drizzle',
  'cloud-lightning',
  'cloud-off',
  'cloud-rain',
  'cloud-snow',
  'code',
  'codepen',
  'codesandbox',
  'coffee',
  'columns',
  'command',
  'compass',
  'copy',
  'corner-down-left',
  'corner-down-right',
  'corner-left-down',
  'corner-left-up',
  'corner-right-down',
  'corner-right-up',
  'corner-up-left',
  'corner-up-right',
  'cpu',
  'credit-card',
  'crop',
  'crosshair',
  'database',
  'delete',
  'disc',
  'divide',
  'divide-circle',
  'divide-square',
  'dollar-sign',
  'download',
  'download-cloud',
  'dribbble',
  'droplet',
  'edit',
  'edit-2',
  'edit-3',
  'external-link',
  'eye',
  'eye-off',
  'facebook',
  'fast-forward',
  'feather',
  'figma',
  'file',
  'file-minus',
  'file-plus',
  'file-text',
  'film',
  'filter',
  'flag',
  'folder',
  'folder-minus',
  'folder-plus',
  'framer',
  'frown',
  'gift',
  'git-branch',
  'git-commit',
  'git-merge',
  'git-pull-request',
  'github',
  'gitlab',
  'globe',
  'grid',
  'hard-drive',
  'hash',
  'headphones',
  'heart',
  'help-circle',
  'hexagon',
  'home',
  'image',
  'inbox',
  'info',
  'instagram',
  'italic',
  'key',
  'layers',
  'layout',
  'life-buoy',
  'link',
  'link-2',
  'linkedin',
  'list',
  'loader',
  'lock',
  'log-in',
  'log-out',
  'mail',
  'map',
  'map-pin',
  'maximize',
  'maximize-2',
  'meh',
  'menu',
  'message-circle',
  'message-square',
  'mic',
  'mic-off',
  'minimize',
  'minimize-2',
  'minus',
  'minus-circle',
  'minus-square',
  'monitor',
  'moon',
  'more-horizontal',
  'more-vertical',
  'mouse-pointer',
  'move',
  'music',
  'navigation',
  'navigation-2',
  'octagon',
  'package',
  'paperclip',
  'pause',
  'pause-circle',
  'pen-tool',
  'percent',
  'phone',
  'phone-call',
  'phone-forwarded',
  'phone-incoming',
  'phone-missed',
  'phone-off',
  'phone-outgoing',
  'pie-chart',
  'play',
  'play-circle',
  'plus',
  'plus-circle',
  'plus-square',
  'pocket',
  'power',
  'printer',
  'radio',
  'refresh-ccw',
  'refresh-cw',
  'repeat',
  'rewind',
  'rotate-ccw',
  'rotate-cw',
  'rss',
  'save',
  'scissors',
  'search',
  'send',
  'server',
  'settings',
  'share',
  'share-2',
  'shield',
  'shield-off',
  'shopping-bag',
  'shopping-cart',
  'shuffle',
  'sidebar',
  'skip-back',
  'skip-forward',
  'slack',
  'slash',
  'sliders',
  'smartphone',
  'smile',
  'speaker',
  'square',
  'star',
  'stop-circle',
  'sun',
  'sunrise',
  'sunset',
  'tablet',
  'tag',
  'target',
  'terminal',
  'thermometer',
  'thumbs-down',
  'thumbs-up',
  'toggle-left',
  'toggle-right',
  'tool',
  'trash',
  'trash-2',
  'trello',
  'trending-down',
  'trending-up',
  'triangle',
  'truck',
  'tv',
  'twitch',
  'twitter',
  'type',
  'umbrella',
  'underline',
  'unlock',
  'upload',
  'upload-cloud',
  'user',
  'user-check',
  'user-minus',
  'user-plus',
  'user-x',
  'users',
  'video',
  'video-off',
  'voicemail',
  'volume',
  'volume-1',
  'volume-2',
  'volume-x',
  'watch',
  'wifi',
  'wifi-off',
  'wind',
  'x',
  'x-circle',
  'x-octagon',
  'x-square',
  'youtube',
  'zap',
  'zap-off',
  'zoom-in',
  'zoom-out',
];
var moduleBaseUrl = 'https://framer.com/m/feather-icons/';
var uppercaseIconKeys = iconKeys.map((name,) => name.charAt(0,).toUpperCase() + name.slice(1,));
var lowercaseIconKeyPairs = iconKeys.reduce((res, key,) => {
  res[key.toLowerCase()] = key;
  return res;
}, {},);
function Icon(props,) {
  const { color, selectByList, iconSearch, iconSelection, onClick, onMouseDown, onMouseUp, onMouseEnter, onMouseLeave, mirrored, } = props;
  const isMounted = useRef(false,);
  const iconKey = useIconSelection(iconKeys, selectByList, iconSearch, iconSelection, lowercaseIconKeyPairs,);
  const [SelectedIcon, setSelectedIcon,] = useState(iconKey === 'Home' ? s(React,) : null,);
  async function importModule() {
    let active = true;
    try {
      const iconModuleUrl = `${moduleBaseUrl}${iconKey}.js@0.0.29`;
      const module = await import(
        /* webpackIgnore: true */
        /* @vite-ignore */
        iconModuleUrl
      );
      if (active) setSelectedIcon(module.default(React,),);
    } catch (e) {
      console.log(e,);
      if (active) setSelectedIcon(null,);
    }
    return () => {
      active = false;
    };
  }
  useEffect(() => {
    importModule();
  }, [iconKey,],);
  const isOnCanvas = RenderTarget.current() === RenderTarget.canvas;
  const emptyState = isOnCanvas ? /* @__PURE__ */ _jsx(NullState, {},) : null;
  return /* @__PURE__ */ _jsx('div', {
    style: { display: 'contents', },
    onClick,
    onMouseEnter,
    onMouseLeave,
    onMouseDown,
    onMouseUp,
    children: SelectedIcon
      ? /* @__PURE__ */ _jsx(SelectedIcon, {
        style: { width: '100%', height: '100%', transform: mirrored ? 'scale(-1, 1)' : void 0, },
        color,
      },)
      : emptyState,
  },);
}
Icon.displayName = 'Feather';
Icon.defaultProps = {
  width: 24,
  height: 24,
  iconSelection: 'home',
  iconSearch: 'Home',
  color: '#66F',
  selectByList: true,
  mirrored: false,
};
addPropertyControls(Icon, {
  selectByList: {
    type: ControlType.Boolean,
    title: 'Select',
    enabledTitle: 'List',
    disabledTitle: 'Search',
    defaultValue: Icon.defaultProps.selectByList,
  },
  iconSelection: {
    type: ControlType.Enum,
    options: iconKeys,
    optionTitles: uppercaseIconKeys,
    defaultValue: Icon.defaultProps.iconSelection,
    title: 'Name',
    hidden: ({ selectByList, },) => !selectByList,
    description: 'Find every icon name on the [Feather site](https://feathericons.com/)',
  },
  iconSearch: {
    type: ControlType.String,
    title: 'Name',
    placeholder: 'Menu, Wifi, Box\u2026',
    hidden: ({ selectByList, },) => selectByList,
  },
  mirrored: { type: ControlType.Boolean, enabledTitle: 'Yes', disabledTitle: 'No', defaultValue: Icon.defaultProps.mirrored, },
  color: { type: ControlType.Color, title: 'Color', defaultValue: Icon.defaultProps.color, },
  ...defaultEvents,
},);

// https :https://framerusercontent.com/modules/lHvu07oBqQud5n3tAkGN/NeGEkXadMrfTiyQajVxV/M5VCp5TIr.js
import { jsx as _jsx2, jsxs as _jsxs, } from 'react/jsx-runtime';
import {
  addFonts,
  addPropertyControls as addPropertyControls2,
  ControlType as ControlType2,
  cx,
  RichText,
  SVG,
  useActiveVariantCallback,
  useComponentViewport,
  useLocaleInfo,
  useVariantState,
  withCSS,
} from 'unframer';
import { LayoutGroup, motion, MotionConfigContext, } from 'unframer';
import * as React2 from 'react';
var cycleOrder = ['cOu2SK2RI', 'bomnfqW1D', 'rGhfB3H3W', 'vrs7500H0',];
var serializationHash = 'framer-3trHt';
var variantClassNames = {
  bomnfqW1D: 'framer-v-1em6qc1',
  cOu2SK2RI: 'framer-v-7uan49',
  rGhfB3H3W: 'framer-v-mwo4kg',
  vrs7500H0: 'framer-v-sbgjd',
};
function addPropertyOverrides(overrides, ...variants) {
  const nextOverrides = {};
  variants === null || variants === void 0
    ? void 0
    : variants.forEach((variant,) => variant && Object.assign(nextOverrides, overrides[variant],));
  return nextOverrides;
}
var transition1 = { bounce: 0.2, delay: 0, duration: 0.4, type: 'spring', };
var Transition = ({ value, children, },) => {
  const config = React2.useContext(MotionConfigContext,);
  const transition = value !== null && value !== void 0 ? value : config.transition;
  const contextValue = React2.useMemo(() => ({ ...config, transition, }), [JSON.stringify(transition,),],);
  return /* @__PURE__ */ _jsx2(MotionConfigContext.Provider, { value: contextValue, children, },);
};
var Variants = motion.create(React2.Fragment,);
var humanReadableVariantMap = { 'Variant 1': 'cOu2SK2RI', 'Variant 2': 'bomnfqW1D', 'Variant 3': 'rGhfB3H3W', 'Variant 4': 'vrs7500H0', };
var getProps = ({ height, id, width, ...props },) => {
  var _humanReadableVariantMap_props_variant, _ref;
  return {
    ...props,
    variant:
      (_ref =
            (_humanReadableVariantMap_props_variant = humanReadableVariantMap[props.variant]) !== null &&
              _humanReadableVariantMap_props_variant !== void 0
              ? _humanReadableVariantMap_props_variant
              : props.variant) !== null && _ref !== void 0
        ? _ref
        : 'cOu2SK2RI',
  };
};
var createLayoutDependency = (props, variants,) => {
  if (props.layoutDependency) return variants.join('-',) + props.layoutDependency;
  return variants.join('-',);
};
var Component = /* @__PURE__ */ React2.forwardRef(function (props, ref,) {
  const { activeLocale, setLocale, } = useLocaleInfo();
  const { style, className: className3, layoutId, variant, ...restProps } = getProps(props,);
  const {
    baseVariant,
    classNames,
    clearLoadingGesture,
    gestureHandlers,
    gestureVariant,
    isLoading,
    setGestureState,
    setVariant,
    variants,
  } = useVariantState({ cycleOrder, defaultVariant: 'cOu2SK2RI', variant, variantClassNames, },);
  const layoutDependency = createLayoutDependency(props, variants,);
  const { activeVariantCallback, delay, } = useActiveVariantCallback(baseVariant,);
  const onTap1e02gdc = activeVariantCallback(async (...args) => {
    setVariant('cOu2SK2RI',);
  },);
  const onTap1hvevos = activeVariantCallback(async (...args) => {
    setVariant('bomnfqW1D',);
  },);
  const onTap10ly6xe = activeVariantCallback(async (...args) => {
    setVariant('rGhfB3H3W',);
  },);
  const onTapv7mde6 = activeVariantCallback(async (...args) => {
    setVariant('vrs7500H0',);
  },);
  const ref1 = React2.useRef(null,);
  const isDisplayed = () => {
    if (['bomnfqW1D', 'rGhfB3H3W', 'vrs7500H0',].includes(baseVariant,)) return false;
    return true;
  };
  const isDisplayed1 = () => {
    if (['bomnfqW1D', 'rGhfB3H3W', 'vrs7500H0',].includes(baseVariant,)) return true;
    return false;
  };
  const defaultLayoutId = React2.useId();
  const sharedStyleClassNames = [];
  const componentViewport = useComponentViewport();
  return /* @__PURE__ */ _jsx2(LayoutGroup, {
    id: layoutId !== null && layoutId !== void 0 ? layoutId : defaultLayoutId,
    children: /* @__PURE__ */ _jsx2(Variants, {
      animate: variants,
      initial: false,
      children: /* @__PURE__ */ _jsx2(Transition, {
        value: transition1,
        children: /* @__PURE__ */ _jsxs(motion.div, {
          ...restProps,
          ...gestureHandlers,
          className: cx(serializationHash, ...sharedStyleClassNames, 'framer-7uan49', className3, classNames,),
          'data-framer-name': 'Variant 1',
          layoutDependency,
          layoutId: 'cOu2SK2RI',
          ref: ref !== null && ref !== void 0 ? ref : ref1,
          style: { ...style, },
          ...addPropertyOverrides(
            {
              bomnfqW1D: { 'data-framer-name': 'Variant 2', },
              rGhfB3H3W: { 'data-framer-name': 'Variant 3', },
              vrs7500H0: { 'data-framer-name': 'Variant 4', },
            },
            baseVariant,
            gestureVariant,
          ),
          children: [
            /* @__PURE__ */ _jsxs(motion.div, {
              className: 'framer-1u67sjh',
              layoutDependency,
              layoutId: 'XXq3BLwwk',
              children: [
                /* @__PURE__ */ _jsx2(RichText, {
                  __fromCanvasComponent: true,
                  children: /* @__PURE__ */ _jsx2(React2.Fragment, {
                    children: /* @__PURE__ */ _jsx2(motion.h3, {
                      style: {
                        '--font-selector': 'SW50ZXItQm9sZA==',
                        '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                        '--framer-font-size': '24px',
                        '--framer-font-weight': '700',
                        '--framer-letter-spacing': '-1.9px',
                        '--framer-text-alignment': 'left',
                        '--framer-text-color': 'var(--extracted-a0htzi, rgb(255, 255, 255))',
                      },
                      children: 'Dedicated Hardware',
                    },),
                  },),
                  className: 'framer-1sm3lsh',
                  fonts: ['Inter-Bold',],
                  layoutDependency,
                  layoutId: 'Htz7vRu6y',
                  style: {
                    '--extracted-a0htzi': 'rgb(255, 255, 255)',
                    '--framer-link-text-color': 'rgb(0, 153, 255)',
                    '--framer-link-text-decoration': 'underline',
                    '--framer-paragraph-spacing': '0px',
                  },
                  verticalAlignment: 'top',
                  withExternalLayout: true,
                },),
                /* @__PURE__ */ _jsx2(RichText, {
                  __fromCanvasComponent: true,
                  children: /* @__PURE__ */ _jsx2(React2.Fragment, {
                    children: /* @__PURE__ */ _jsx2(motion.p, {
                      style: { '--framer-text-alignment': 'left', '--framer-text-color': 'var(--extracted-r6o4lv, rgb(138, 138, 138))', },
                      children: 'For applications that demand high performance',
                    },),
                  },),
                  className: 'framer-1lu0kyq',
                  fonts: ['Inter',],
                  layoutDependency,
                  layoutId: 't4_MQM43Z',
                  style: { '--extracted-r6o4lv': 'rgb(138, 138, 138)', },
                  verticalAlignment: 'top',
                  withExternalLayout: true,
                },),
              ],
            },),
            /* @__PURE__ */ _jsxs(motion.div, {
              className: 'framer-1lh4l1f',
              layoutDependency,
              layoutId: 'z9nTAslKo',
              children: [
                /* @__PURE__ */ _jsx2(motion.div, {
                  className: 'framer-19dbrh6',
                  'data-border': true,
                  'data-highlight': true,
                  layoutDependency,
                  layoutId: 'JDvxK2IeP',
                  onTap: onTap1e02gdc,
                  style: {
                    '--border-bottom-width': '1px',
                    '--border-color': 'var(--token-f94bc001-f1ab-463c-abc3-37fb7e541046, rgb(255, 79, 1))',
                    '--border-left-width': '1px',
                    '--border-right-width': '1px',
                    '--border-style': 'solid',
                    '--border-top-width': '1px',
                    borderBottomLeftRadius: 3,
                    borderBottomRightRadius: 3,
                    borderTopLeftRadius: 3,
                    borderTopRightRadius: 3,
                  },
                  variants: {
                    bomnfqW1D: {
                      '--border-bottom-width': '0px',
                      '--border-left-width': '0px',
                      '--border-right-width': '0px',
                      '--border-top-width': '0px',
                    },
                    rGhfB3H3W: {
                      '--border-bottom-width': '0px',
                      '--border-left-width': '0px',
                      '--border-right-width': '0px',
                      '--border-top-width': '0px',
                    },
                    vrs7500H0: {
                      '--border-bottom-width': '0px',
                      '--border-left-width': '0px',
                      '--border-right-width': '0px',
                      '--border-top-width': '0px',
                    },
                  },
                  children: /* @__PURE__ */ _jsx2(RichText, {
                    __fromCanvasComponent: true,
                    children: /* @__PURE__ */ _jsx2(React2.Fragment, {
                      children: /* @__PURE__ */ _jsx2(motion.p, {
                        style: {
                          '--font-selector': 'SW50ZXItQm9sZA==',
                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                          '--framer-font-weight': '700',
                          '--framer-text-alignment': 'center',
                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                        },
                        children: 'Community',
                      },),
                    },),
                    className: 'framer-1a84utr',
                    fonts: ['Inter-Bold',],
                    layoutDependency,
                    layoutId: 'MqC8a2xP1',
                    style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                    verticalAlignment: 'center',
                    withExternalLayout: true,
                  },),
                },),
                /* @__PURE__ */ _jsx2(motion.div, {
                  className: 'framer-6kr5gq',
                  'data-highlight': true,
                  layoutDependency,
                  layoutId: 'aSrTpfefG',
                  onTap: onTap1hvevos,
                  style: {
                    '--border-bottom-width': '0px',
                    '--border-color': 'rgba(0, 0, 0, 0)',
                    '--border-left-width': '0px',
                    '--border-right-width': '0px',
                    '--border-style': 'solid',
                    '--border-top-width': '0px',
                    borderBottomLeftRadius: 0,
                    borderBottomRightRadius: 0,
                    borderTopLeftRadius: 0,
                    borderTopRightRadius: 0,
                  },
                  variants: {
                    bomnfqW1D: {
                      '--border-bottom-width': '1px',
                      '--border-color': 'var(--token-f94bc001-f1ab-463c-abc3-37fb7e541046, rgb(255, 79, 1))',
                      '--border-left-width': '1px',
                      '--border-right-width': '1px',
                      '--border-style': 'solid',
                      '--border-top-width': '1px',
                      borderBottomLeftRadius: 3,
                      borderBottomRightRadius: 3,
                      borderTopLeftRadius: 3,
                      borderTopRightRadius: 3,
                    },
                    rGhfB3H3W: { borderBottomLeftRadius: 3, borderBottomRightRadius: 3, borderTopLeftRadius: 3, borderTopRightRadius: 3, },
                    vrs7500H0: { borderBottomLeftRadius: 3, borderBottomRightRadius: 3, borderTopLeftRadius: 3, borderTopRightRadius: 3, },
                  },
                  ...addPropertyOverrides({ bomnfqW1D: { 'data-border': true, }, }, baseVariant, gestureVariant,),
                  children: /* @__PURE__ */ _jsx2(RichText, {
                    __fromCanvasComponent: true,
                    children: /* @__PURE__ */ _jsx2(React2.Fragment, {
                      children: /* @__PURE__ */ _jsx2(motion.p, {
                        style: {
                          '--font-selector': 'SW50ZXItQm9sZA==',
                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                          '--framer-font-weight': '700',
                          '--framer-text-alignment': 'center',
                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                        },
                        children: 'Pro',
                      },),
                    },),
                    className: 'framer-1xfr7n6',
                    fonts: ['Inter-Bold',],
                    layoutDependency,
                    layoutId: 'qNYrTMXO9',
                    style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                    verticalAlignment: 'center',
                    withExternalLayout: true,
                  },),
                },),
                /* @__PURE__ */ _jsx2(motion.div, {
                  className: 'framer-9nl8kx',
                  'data-highlight': true,
                  layoutDependency,
                  layoutId: 'AjEsdOTQA',
                  onTap: onTap10ly6xe,
                  style: {
                    '--border-bottom-width': '0px',
                    '--border-color': 'rgba(0, 0, 0, 0)',
                    '--border-left-width': '0px',
                    '--border-right-width': '0px',
                    '--border-style': 'solid',
                    '--border-top-width': '0px',
                    borderBottomLeftRadius: 0,
                    borderBottomRightRadius: 0,
                    borderTopLeftRadius: 0,
                    borderTopRightRadius: 0,
                  },
                  variants: {
                    rGhfB3H3W: {
                      '--border-bottom-width': '1px',
                      '--border-color': 'var(--token-f94bc001-f1ab-463c-abc3-37fb7e541046, rgb(255, 79, 1))',
                      '--border-left-width': '1px',
                      '--border-right-width': '1px',
                      '--border-style': 'solid',
                      '--border-top-width': '1px',
                      borderBottomLeftRadius: 3,
                      borderBottomRightRadius: 3,
                      borderTopLeftRadius: 3,
                      borderTopRightRadius: 3,
                    },
                    vrs7500H0: { borderBottomLeftRadius: 3, borderBottomRightRadius: 3, borderTopLeftRadius: 3, borderTopRightRadius: 3, },
                  },
                  ...addPropertyOverrides({ rGhfB3H3W: { 'data-border': true, }, }, baseVariant, gestureVariant,),
                  children: /* @__PURE__ */ _jsx2(RichText, {
                    __fromCanvasComponent: true,
                    children: /* @__PURE__ */ _jsx2(React2.Fragment, {
                      children: /* @__PURE__ */ _jsx2(motion.p, {
                        style: {
                          '--font-selector': 'SW50ZXItQm9sZA==',
                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                          '--framer-font-weight': '700',
                          '--framer-text-alignment': 'center',
                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                        },
                        children: 'Team',
                      },),
                    },),
                    className: 'framer-mjoufp',
                    fonts: ['Inter-Bold',],
                    layoutDependency,
                    layoutId: 'iZW2fvp9k',
                    style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                    verticalAlignment: 'center',
                    withExternalLayout: true,
                  },),
                },),
                /* @__PURE__ */ _jsx2(motion.div, {
                  className: 'framer-kxg30q',
                  'data-highlight': true,
                  layoutDependency,
                  layoutId: 'Al0myv9Bc',
                  onTap: onTapv7mde6,
                  style: {
                    '--border-bottom-width': '0px',
                    '--border-color': 'rgba(0, 0, 0, 0)',
                    '--border-left-width': '0px',
                    '--border-right-width': '0px',
                    '--border-style': 'solid',
                    '--border-top-width': '0px',
                    borderBottomLeftRadius: 0,
                    borderBottomRightRadius: 0,
                    borderTopLeftRadius: 0,
                    borderTopRightRadius: 0,
                  },
                  variants: {
                    vrs7500H0: {
                      '--border-bottom-width': '1px',
                      '--border-color': 'var(--token-f94bc001-f1ab-463c-abc3-37fb7e541046, rgb(255, 79, 1))',
                      '--border-left-width': '1px',
                      '--border-right-width': '1px',
                      '--border-style': 'solid',
                      '--border-top-width': '1px',
                      borderBottomLeftRadius: 3,
                      borderBottomRightRadius: 3,
                      borderTopLeftRadius: 3,
                      borderTopRightRadius: 3,
                    },
                  },
                  ...addPropertyOverrides({ vrs7500H0: { 'data-border': true, }, }, baseVariant, gestureVariant,),
                  children: /* @__PURE__ */ _jsx2(RichText, {
                    __fromCanvasComponent: true,
                    children: /* @__PURE__ */ _jsx2(React2.Fragment, {
                      children: /* @__PURE__ */ _jsx2(motion.p, {
                        style: {
                          '--font-selector': 'SW50ZXItQm9sZA==',
                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                          '--framer-font-weight': '700',
                          '--framer-text-alignment': 'center',
                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                        },
                        children: 'Enterprise',
                      },),
                    },),
                    className: 'framer-z8z3ut',
                    fonts: ['Inter-Bold',],
                    layoutDependency,
                    layoutId: 'MCAFE8a72',
                    style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                    verticalAlignment: 'center',
                    withExternalLayout: true,
                  },),
                },),
              ],
            },),
            /* @__PURE__ */ _jsxs(motion.div, {
              className: 'framer-10xkmt7',
              'data-border': true,
              layoutDependency,
              layoutId: 'reMAQV6nf',
              style: {
                '--border-bottom-width': '0px',
                '--border-color': 'var(--token-be2928fe-8496-42f6-9733-c37829997236, rgb(33, 33, 33))',
                '--border-left-width': '0px',
                '--border-right-width': '0px',
                '--border-style': 'solid',
                '--border-top-width': '1px',
              },
              children: [
                /* @__PURE__ */ _jsx2(motion.div, {
                  className: 'framer-17dwfkv',
                  layoutDependency,
                  layoutId: 'pT9ZJFlJv',
                  style: { borderBottomLeftRadius: 3, borderBottomRightRadius: 3, borderTopLeftRadius: 3, borderTopRightRadius: 3, },
                  children: /* @__PURE__ */ _jsx2(RichText, {
                    __fromCanvasComponent: true,
                    children: /* @__PURE__ */ _jsx2(React2.Fragment, {
                      children: /* @__PURE__ */ _jsx2(motion.p, {
                        style: {
                          '--font-selector': 'SW50ZXItQm9sZA==',
                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                          '--framer-font-weight': '700',
                          '--framer-text-alignment': 'left',
                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                        },
                        children: 'Any Region',
                      },),
                    },),
                    className: 'framer-ewnbyf',
                    fonts: ['Inter-Bold',],
                    layoutDependency,
                    layoutId: 'R5YQCNmQK',
                    style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                    verticalAlignment: 'center',
                    withExternalLayout: true,
                  },),
                },),
                /* @__PURE__ */ _jsx2(motion.div, {
                  className: 'framer-b0q0ki',
                  layoutDependency,
                  layoutId: 'aib4XbXBY',
                  children: /* @__PURE__ */ _jsxs(motion.div, {
                    className: 'framer-1imdwud',
                    layoutDependency,
                    layoutId: 'HCzTUM3dJ',
                    style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                    children: [
                      isDisplayed() && /* @__PURE__ */ _jsx2(RichText, {
                        __fromCanvasComponent: true,
                        children: /* @__PURE__ */ _jsx2(React2.Fragment, {
                          children: /* @__PURE__ */ _jsx2(motion.p, {
                            style: {
                              '--framer-text-alignment': 'center',
                              '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                            },
                            children: '-',
                          },),
                        },),
                        className: 'framer-ry26qm',
                        fonts: ['Inter',],
                        layoutDependency,
                        layoutId: 'HLn8R1RUb',
                        style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                        verticalAlignment: 'top',
                        withExternalLayout: true,
                      },),
                      isDisplayed1() && /* @__PURE__ */ _jsx2(SVG, {
                        className: 'framer-15rr4o2',
                        'data-framer-name': 'Check',
                        layout: 'position',
                        layoutDependency,
                        layoutId: 'DcVdc14BN',
                        opacity: 1,
                        radius: 0,
                        style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                        svg:
                          '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 24 24"><path d="M 12 2.25 C 6.615 2.25 2.25 6.615 2.25 12 C 2.25 17.385 6.615 21.75 12 21.75 C 17.385 21.75 21.75 17.385 21.75 12 C 21.74 6.62 17.38 2.26 12 2.25 Z M 16.641 10.294 L 11.147 15.544 C 11.005 15.677 10.817 15.751 10.622 15.75 C 10.429 15.753 10.244 15.679 10.106 15.544 L 7.359 12.919 C 7.152 12.738 7.06 12.458 7.121 12.189 C 7.181 11.92 7.384 11.706 7.649 11.632 C 7.914 11.557 8.199 11.634 8.391 11.831 L 10.622 13.959 L 15.609 9.206 C 15.912 8.942 16.37 8.964 16.647 9.255 C 16.923 9.547 16.921 10.005 16.641 10.294 Z" fill="rgb(255, 79, 1)"></path></svg>',
                        svgContentId: 11856876933,
                        withExternalLayout: true,
                        ...addPropertyOverrides(
                          {
                            bomnfqW1D: { svgContentId: 10285337634, },
                            rGhfB3H3W: { svgContentId: 10285337634, },
                            vrs7500H0: { svgContentId: 10285337634, },
                          },
                          baseVariant,
                          gestureVariant,
                        ),
                      },),
                    ],
                  },),
                },),
                /* @__PURE__ */ _jsx2(motion.div, {
                  className: 'framer-18z5va3',
                  layoutDependency,
                  layoutId: 'P_dR_My5G',
                  children: /* @__PURE__ */ _jsx2(RichText, {
                    __fromCanvasComponent: true,
                    children: /* @__PURE__ */ _jsx2(React2.Fragment, {
                      children: /* @__PURE__ */ _jsx2(motion.p, {
                        style: {
                          '--font-selector': 'R0Y7SW50ZXItNzAw',
                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                          '--framer-font-weight': '700',
                          '--framer-text-alignment': 'left',
                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                        },
                        children: 'Providers',
                      },),
                    },),
                    className: 'framer-wz98hc',
                    fonts: ['GF;Inter-700',],
                    layoutDependency,
                    layoutId: 'Z7_KMuNHJ',
                    style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                    verticalAlignment: 'center',
                    withExternalLayout: true,
                  },),
                },),
                /* @__PURE__ */ _jsx2(motion.div, {
                  className: 'framer-ckzv22',
                  layoutDependency,
                  layoutId: 'avmiBiFMC',
                  children: /* @__PURE__ */ _jsx2(motion.div, {
                    className: 'framer-jj0idf',
                    layoutDependency,
                    layoutId: 'lvINCy0Im',
                    style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                    children: /* @__PURE__ */ _jsx2(RichText, {
                      __fromCanvasComponent: true,
                      children: /* @__PURE__ */ _jsx2(React2.Fragment, {
                        children: /* @__PURE__ */ _jsx2(motion.p, {
                          style: {
                            '--framer-text-alignment': 'center',
                            '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                          },
                          children: '-',
                        },),
                      },),
                      className: 'framer-15i5s5p',
                      fonts: ['Inter',],
                      layoutDependency,
                      layoutId: 'QW6BG4dzK',
                      style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                      verticalAlignment: 'top',
                      withExternalLayout: true,
                      ...addPropertyOverrides(
                        {
                          bomnfqW1D: {
                            children: /* @__PURE__ */ _jsx2(React2.Fragment, {
                              children: /* @__PURE__ */ _jsx2(motion.p, {
                                style: {
                                  '--framer-text-alignment': 'center',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                                },
                                children: 'Linode',
                              },),
                            },),
                          },
                          rGhfB3H3W: {
                            children: /* @__PURE__ */ _jsx2(React2.Fragment, {
                              children: /* @__PURE__ */ _jsx2(motion.p, {
                                style: {
                                  '--framer-text-alignment': 'center',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                                },
                                children: 'Linode',
                              },),
                            },),
                          },
                          vrs7500H0: {
                            children: /* @__PURE__ */ _jsx2(React2.Fragment, {
                              children: /* @__PURE__ */ _jsx2(motion.p, {
                                style: {
                                  '--framer-text-alignment': 'center',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                                },
                                children: 'Custom',
                              },),
                            },),
                          },
                        },
                        baseVariant,
                        gestureVariant,
                      ),
                    },),
                  },),
                },),
                /* @__PURE__ */ _jsx2(RichText, {
                  __fromCanvasComponent: true,
                  children: /* @__PURE__ */ _jsxs(React2.Fragment, {
                    children: [
                      /* @__PURE__ */ _jsx2(motion.p, {
                        style: {
                          '--font-selector': 'R0Y7SW50ZXItNzAw',
                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                          '--framer-font-weight': '700',
                          '--framer-text-alignment': 'left',
                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                        },
                        children: 'Data Center',
                      },),
                      /* @__PURE__ */ _jsx2(motion.p, {
                        style: {
                          '--font-selector': 'R0Y7SW50ZXItNzAw',
                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                          '--framer-font-weight': '700',
                          '--framer-text-alignment': 'left',
                          '--framer-text-color': 'var(--extracted-2gxw0f, rgb(255, 255, 255))',
                        },
                        children: 'Failover',
                      },),
                    ],
                  },),
                  className: 'framer-zvok7z',
                  fonts: ['GF;Inter-700',],
                  layoutDependency,
                  layoutId: 'gYbf4_ZZa',
                  style: { '--extracted-2gxw0f': 'rgb(255, 255, 255)', '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                  verticalAlignment: 'center',
                  withExternalLayout: true,
                },),
                /* @__PURE__ */ _jsxs(motion.div, {
                  className: 'framer-1o7nn7l',
                  layoutDependency,
                  layoutId: 'VeaXk3mec',
                  style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                  children: [
                    isDisplayed() && /* @__PURE__ */ _jsx2(RichText, {
                      __fromCanvasComponent: true,
                      children: /* @__PURE__ */ _jsx2(React2.Fragment, {
                        children: /* @__PURE__ */ _jsx2(motion.p, {
                          style: {
                            '--framer-text-alignment': 'center',
                            '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                          },
                          children: '-',
                        },),
                      },),
                      className: 'framer-muczq9',
                      fonts: ['Inter',],
                      layoutDependency,
                      layoutId: 'kEmftUL7t',
                      style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                      verticalAlignment: 'top',
                      withExternalLayout: true,
                    },),
                    isDisplayed1() && /* @__PURE__ */ _jsx2(SVG, {
                      className: 'framer-ascd9y',
                      'data-framer-name': 'Check',
                      layout: 'position',
                      layoutDependency,
                      layoutId: 'ijnjN1pGa',
                      opacity: 1,
                      radius: 0,
                      style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                      svg:
                        '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 24 24"><path d="M 12 2.25 C 6.615 2.25 2.25 6.615 2.25 12 C 2.25 17.385 6.615 21.75 12 21.75 C 17.385 21.75 21.75 17.385 21.75 12 C 21.74 6.62 17.38 2.26 12 2.25 Z M 16.641 10.294 L 11.147 15.544 C 11.005 15.677 10.817 15.751 10.622 15.75 C 10.429 15.753 10.244 15.679 10.106 15.544 L 7.359 12.919 C 7.152 12.738 7.06 12.458 7.121 12.189 C 7.181 11.92 7.384 11.706 7.649 11.632 C 7.914 11.557 8.199 11.634 8.391 11.831 L 10.622 13.959 L 15.609 9.206 C 15.912 8.942 16.37 8.964 16.647 9.255 C 16.923 9.547 16.921 10.005 16.641 10.294 Z" fill="rgb(255, 79, 1)"></path></svg>',
                      svgContentId: 11856876933,
                      withExternalLayout: true,
                      ...addPropertyOverrides(
                        {
                          bomnfqW1D: { svgContentId: 10285337634, },
                          rGhfB3H3W: { svgContentId: 10285337634, },
                          vrs7500H0: { svgContentId: 10285337634, },
                        },
                        baseVariant,
                        gestureVariant,
                      ),
                    },),
                  ],
                },),
                /* @__PURE__ */ _jsx2(motion.div, {
                  className: 'framer-1xbkss5',
                  layoutDependency,
                  layoutId: 'tU17Cyl74',
                  children: /* @__PURE__ */ _jsx2(RichText, {
                    __fromCanvasComponent: true,
                    children: /* @__PURE__ */ _jsxs(React2.Fragment, {
                      children: [
                        /* @__PURE__ */ _jsx2(motion.p, {
                          style: {
                            '--font-selector': 'R0Y7SW50ZXItNzAw',
                            '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                            '--framer-font-weight': '700',
                            '--framer-text-alignment': 'left',
                            '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                          },
                          children: 'Automatic',
                        },),
                        /* @__PURE__ */ _jsx2(motion.p, {
                          style: {
                            '--font-selector': 'R0Y7SW50ZXItNzAw',
                            '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                            '--framer-font-weight': '700',
                            '--framer-text-alignment': 'left',
                            '--framer-text-color': 'var(--extracted-2gxw0f, rgb(255, 255, 255))',
                          },
                          children: 'SSL',
                        },),
                      ],
                    },),
                    className: 'framer-f5sjir',
                    fonts: ['GF;Inter-700',],
                    layoutDependency,
                    layoutId: 'yUUBM35UH',
                    style: { '--extracted-2gxw0f': 'rgb(255, 255, 255)', '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                    verticalAlignment: 'center',
                    withExternalLayout: true,
                  },),
                },),
                /* @__PURE__ */ _jsx2(motion.div, {
                  className: 'framer-1r28de6',
                  layoutDependency,
                  layoutId: 'tRgRh9LTG',
                  children: /* @__PURE__ */ _jsxs(motion.div, {
                    className: 'framer-16qhmee',
                    layoutDependency,
                    layoutId: 'mLahsKNqx',
                    style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                    children: [
                      isDisplayed() && /* @__PURE__ */ _jsx2(RichText, {
                        __fromCanvasComponent: true,
                        children: /* @__PURE__ */ _jsx2(React2.Fragment, {
                          children: /* @__PURE__ */ _jsx2(motion.p, {
                            style: {
                              '--framer-text-alignment': 'center',
                              '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                            },
                            children: '-',
                          },),
                        },),
                        className: 'framer-1ltk1xr',
                        fonts: ['Inter',],
                        layoutDependency,
                        layoutId: 'dBnqZaLZ5',
                        style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                        verticalAlignment: 'top',
                        withExternalLayout: true,
                      },),
                      isDisplayed1() && /* @__PURE__ */ _jsx2(SVG, {
                        className: 'framer-hgwk4i',
                        'data-framer-name': 'Check',
                        layout: 'position',
                        layoutDependency,
                        layoutId: 'ojs0te4dV',
                        opacity: 1,
                        radius: 0,
                        style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                        svg:
                          '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 24 24"><path d="M 12 2.25 C 6.615 2.25 2.25 6.615 2.25 12 C 2.25 17.385 6.615 21.75 12 21.75 C 17.385 21.75 21.75 17.385 21.75 12 C 21.74 6.62 17.38 2.26 12 2.25 Z M 16.641 10.294 L 11.147 15.544 C 11.005 15.677 10.817 15.751 10.622 15.75 C 10.429 15.753 10.244 15.679 10.106 15.544 L 7.359 12.919 C 7.152 12.738 7.06 12.458 7.121 12.189 C 7.181 11.92 7.384 11.706 7.649 11.632 C 7.914 11.557 8.199 11.634 8.391 11.831 L 10.622 13.959 L 15.609 9.206 C 15.912 8.942 16.37 8.964 16.647 9.255 C 16.923 9.547 16.921 10.005 16.641 10.294 Z" fill="rgb(255, 79, 1)"></path></svg>',
                        svgContentId: 11856876933,
                        withExternalLayout: true,
                        ...addPropertyOverrides(
                          {
                            bomnfqW1D: { svgContentId: 10285337634, },
                            rGhfB3H3W: { svgContentId: 10285337634, },
                            vrs7500H0: { svgContentId: 10285337634, },
                          },
                          baseVariant,
                          gestureVariant,
                        ),
                      },),
                    ],
                  },),
                },),
              ],
            },),
          ],
        },),
      },),
    },),
  },);
},);
var css3 = [
  '@supports (aspect-ratio: 1) { body { --framer-aspect-ratio-supported: auto; } }',
  '.framer-3trHt.framer-1jjkc18, .framer-3trHt .framer-1jjkc18 { display: block; }',
  '.framer-3trHt.framer-7uan49 { align-content: center; align-items: center; display: flex; flex-direction: column; flex-wrap: nowrap; gap: 18px; height: min-content; justify-content: center; overflow: visible; padding: 0px; position: relative; width: 240px; }',
  '.framer-3trHt .framer-1u67sjh { align-content: flex-start; align-items: flex-start; display: flex; flex: none; flex-direction: column; flex-wrap: nowrap; gap: 2px; height: min-content; justify-content: center; overflow: hidden; padding: 0px; position: relative; width: 100%; }',
  '.framer-3trHt .framer-1sm3lsh { flex: none; height: auto; overflow: visible; position: relative; white-space: pre; width: auto; }',
  '.framer-3trHt .framer-1lu0kyq { flex: none; height: auto; position: relative; white-space: pre-wrap; width: 100%; word-break: break-word; word-wrap: break-word; }',
  '.framer-3trHt .framer-1lh4l1f { display: grid; flex: none; gap: 10px; grid-auto-rows: minmax(0, 1fr); grid-template-columns: repeat(2, minmax(50px, 1fr)); grid-template-rows: repeat(2, minmax(0, 1fr)); height: 68px; justify-content: center; overflow: hidden; padding: 0px; position: relative; width: 238px; }',
  '.framer-3trHt .framer-19dbrh6, .framer-3trHt .framer-6kr5gq, .framer-3trHt .framer-9nl8kx, .framer-3trHt .framer-kxg30q { align-self: center; cursor: pointer; flex: none; height: 100%; justify-self: center; position: relative; width: 100%; }',
  '.framer-3trHt .framer-1a84utr, .framer-3trHt .framer-ewnbyf { bottom: 0px; flex: none; height: 100%; left: calc(50.00000000000002% - 100% / 2); position: absolute; white-space: pre-wrap; width: 100%; word-break: break-word; word-wrap: break-word; }',
  '.framer-3trHt .framer-1xfr7n6, .framer-3trHt .framer-mjoufp, .framer-3trHt .framer-z8z3ut, .framer-3trHt .framer-wz98hc, .framer-3trHt .framer-f5sjir { flex: none; height: 100%; left: 0px; position: absolute; top: 0px; white-space: pre-wrap; width: 100%; word-break: break-word; word-wrap: break-word; }',
  '.framer-3trHt .framer-10xkmt7 { display: grid; flex: none; gap: 10px; grid-auto-rows: minmax(0, 1fr); grid-template-columns: repeat(2, minmax(50px, 1fr)); grid-template-rows: repeat(2, minmax(0, 1fr)); height: 224px; justify-content: center; overflow: hidden; padding: 0px; position: relative; width: 238px; }',
  '.framer-3trHt .framer-17dwfkv, .framer-3trHt .framer-b0q0ki, .framer-3trHt .framer-18z5va3, .framer-3trHt .framer-ckzv22, .framer-3trHt .framer-1xbkss5, .framer-3trHt .framer-1r28de6 { align-self: center; flex: none; height: 100%; justify-self: center; position: relative; width: 100%; }',
  '.framer-3trHt .framer-1imdwud, .framer-3trHt .framer-jj0idf, .framer-3trHt .framer-16qhmee { align-content: center; align-items: center; bottom: 0px; display: flex; flex: none; flex-direction: row; flex-wrap: nowrap; gap: 10px; justify-content: center; left: 0px; padding: 0px; position: absolute; right: 0px; top: 0px; }',
  '.framer-3trHt .framer-ry26qm, .framer-3trHt .framer-15i5s5p, .framer-3trHt .framer-muczq9, .framer-3trHt .framer-1ltk1xr { flex: none; height: auto; position: relative; white-space: pre; width: auto; }',
  '.framer-3trHt .framer-15rr4o2, .framer-3trHt .framer-ascd9y, .framer-3trHt .framer-hgwk4i { flex: none; height: 24px; position: relative; width: 24px; }',
  '.framer-3trHt .framer-zvok7z { align-self: start; flex: none; height: 100%; justify-self: start; position: relative; white-space: pre-wrap; width: 100%; word-break: break-word; word-wrap: break-word; }',
  '.framer-3trHt .framer-1o7nn7l { align-content: center; align-items: center; align-self: center; display: flex; flex: none; flex-direction: row; flex-wrap: nowrap; gap: 10px; height: 100%; justify-content: center; justify-self: center; padding: 0px; position: relative; width: 100%; }',
  '@supports (background: -webkit-named-image(i)) and (not (font-palette:dark)) { .framer-3trHt.framer-7uan49, .framer-3trHt .framer-1u67sjh, .framer-3trHt .framer-1imdwud, .framer-3trHt .framer-jj0idf, .framer-3trHt .framer-1o7nn7l, .framer-3trHt .framer-16qhmee { gap: 0px; } .framer-3trHt.framer-7uan49 > * { margin: 0px; margin-bottom: calc(18px / 2); margin-top: calc(18px / 2); } .framer-3trHt.framer-7uan49 > :first-child, .framer-3trHt .framer-1u67sjh > :first-child { margin-top: 0px; } .framer-3trHt.framer-7uan49 > :last-child, .framer-3trHt .framer-1u67sjh > :last-child { margin-bottom: 0px; } .framer-3trHt .framer-1u67sjh > * { margin: 0px; margin-bottom: calc(2px / 2); margin-top: calc(2px / 2); } .framer-3trHt .framer-1imdwud > *, .framer-3trHt .framer-jj0idf > *, .framer-3trHt .framer-1o7nn7l > *, .framer-3trHt .framer-16qhmee > * { margin: 0px; margin-left: calc(10px / 2); margin-right: calc(10px / 2); } .framer-3trHt .framer-1imdwud > :first-child, .framer-3trHt .framer-jj0idf > :first-child, .framer-3trHt .framer-1o7nn7l > :first-child, .framer-3trHt .framer-16qhmee > :first-child { margin-left: 0px; } .framer-3trHt .framer-1imdwud > :last-child, .framer-3trHt .framer-jj0idf > :last-child, .framer-3trHt .framer-1o7nn7l > :last-child, .framer-3trHt .framer-16qhmee > :last-child { margin-right: 0px; } }',
  '.framer-3trHt[data-border="true"]::after, .framer-3trHt [data-border="true"]::after { content: ""; border-width: var(--border-top-width, 0) var(--border-right-width, 0) var(--border-bottom-width, 0) var(--border-left-width, 0); border-color: var(--border-color, none); border-style: var(--border-style, none); width: 100%; height: 100%; position: absolute; box-sizing: border-box; left: 0; top: 0; border-radius: inherit; pointer-events: none; }',
];
var FramerM5VCp5TIr = withCSS(Component, css3, 'framer-3trHt',);
var stdin_default = FramerM5VCp5TIr;
FramerM5VCp5TIr.displayName = 'Dedicated Mobile';
FramerM5VCp5TIr.defaultProps = { height: 397, width: 240, };
addPropertyControls2(FramerM5VCp5TIr, {
  variant: {
    options: ['cOu2SK2RI', 'bomnfqW1D', 'rGhfB3H3W', 'vrs7500H0',],
    optionTitles: ['Variant 1', 'Variant 2', 'Variant 3', 'Variant 4',],
    title: 'Variant',
    type: ControlType2.Enum,
  },
},);
addFonts(FramerM5VCp5TIr, [{
  explicitInter: true,
  fonts: [{
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0460-052F, U+1C80-1C88, U+20B4, U+2DE0-2DFF, U+A640-A69F, U+FE2E-FE2F',
    url: 'https://framerusercontent.com/assets/DpPBYI0sL4fYLgAkX8KXOPVt7c.woff2',
    weight: '700',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0301, U+0400-045F, U+0490-0491, U+04B0-04B1, U+2116',
    url: 'https://framerusercontent.com/assets/4RAEQdEOrcnDkhHiiCbJOw92Lk.woff2',
    weight: '700',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+1F00-1FFF',
    url: 'https://framerusercontent.com/assets/1K3W8DizY3v4emK8Mb08YHxTbs.woff2',
    weight: '700',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0370-03FF',
    url: 'https://framerusercontent.com/assets/tUSCtfYVM1I1IchuyCwz9gDdQ.woff2',
    weight: '700',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0100-024F, U+0259, U+1E00-1EFF, U+2020, U+20A0-20AB, U+20AD-20CF, U+2113, U+2C60-2C7F, U+A720-A7FF',
    url: 'https://framerusercontent.com/assets/VgYFWiwsAC5OYxAycRXXvhze58.woff2',
    weight: '700',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange:
      'U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+2000-206F, U+2074, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD',
    url: 'https://framerusercontent.com/assets/DXD0Q7LSl7HEvDzucnyLnGBHM.woff2',
    weight: '700',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0102-0103, U+0110-0111, U+0128-0129, U+0168-0169, U+01A0-01A1, U+01AF-01B0, U+1EA0-1EF9, U+20AB',
    url: 'https://framerusercontent.com/assets/GIryZETIX4IFypco5pYZONKhJIo.woff2',
    weight: '700',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0460-052F, U+1C80-1C88, U+20B4, U+2DE0-2DFF, U+A640-A69F, U+FE2E-FE2F',
    url: 'https://framerusercontent.com/assets/5vvr9Vy74if2I6bQbJvbw7SY1pQ.woff2',
    weight: '400',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0301, U+0400-045F, U+0490-0491, U+04B0-04B1, U+2116',
    url: 'https://framerusercontent.com/assets/EOr0mi4hNtlgWNn9if640EZzXCo.woff2',
    weight: '400',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+1F00-1FFF',
    url: 'https://framerusercontent.com/assets/Y9k9QrlZAqio88Klkmbd8VoMQc.woff2',
    weight: '400',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0370-03FF',
    url: 'https://framerusercontent.com/assets/OYrD2tBIBPvoJXiIHnLoOXnY9M.woff2',
    weight: '400',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0100-024F, U+0259, U+1E00-1EFF, U+2020, U+20A0-20AB, U+20AD-20CF, U+2113, U+2C60-2C7F, U+A720-A7FF',
    url: 'https://framerusercontent.com/assets/JeYwfuaPfZHQhEG8U5gtPDZ7WQ.woff2',
    weight: '400',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange:
      'U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+2000-206F, U+2074, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD',
    url: 'https://framerusercontent.com/assets/vQyevYAyHtARFwPqUzQGpnDs.woff2',
    weight: '400',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0102-0103, U+0110-0111, U+0128-0129, U+0168-0169, U+01A0-01A1, U+01AF-01B0, U+1EA0-1EF9, U+20AB',
    url: 'https://framerusercontent.com/assets/b6Y37FthZeALduNqHicBT6FutY.woff2',
    weight: '400',
  }, {
    family: 'Inter',
    source: 'google',
    style: 'normal',
    url: 'https://fonts.gstatic.com/s/inter/v18/UcCO3FwrK3iLTeHuS_nVMrMxCp50SjIw2boKoduKmMEVuFuYMZ1rib2Bg-4.woff2',
    weight: '700',
  },],
},], { supportsExplicitInterCodegen: true, },);

// https :https://framerusercontent.com/modules/QkK7sqwQ3xI6c60xjMCQ/Q051W0ZiHj797iKvK3NJ/sjRZudCNs.js
import { jsx as _jsx3, jsxs as _jsxs2, } from 'react/jsx-runtime';
import {
  addFonts as addFonts2,
  addPropertyControls as addPropertyControls3,
  ControlType as ControlType3,
  cx as cx2,
  RichText as RichText2,
  SVG as SVG2,
  useActiveVariantCallback as useActiveVariantCallback2,
  useComponentViewport as useComponentViewport2,
  useLocaleInfo as useLocaleInfo2,
  useVariantState as useVariantState2,
  withCSS as withCSS2,
} from 'unframer';
import { LayoutGroup as LayoutGroup2, motion as motion2, MotionConfigContext as MotionConfigContext2, } from 'unframer';
import * as React3 from 'react';
var cycleOrder2 = ['GnsxM81Tp', 'F89hmWHtR', 'AwswLpG87', 'cNzZOcKSB',];
var serializationHash2 = 'framer-lsPSX';
var variantClassNames2 = {
  AwswLpG87: 'framer-v-15so1dh',
  cNzZOcKSB: 'framer-v-7tykdh',
  F89hmWHtR: 'framer-v-1r7esh6',
  GnsxM81Tp: 'framer-v-1ct4jx9',
};
function addPropertyOverrides2(overrides, ...variants) {
  const nextOverrides = {};
  variants === null || variants === void 0
    ? void 0
    : variants.forEach((variant,) => variant && Object.assign(nextOverrides, overrides[variant],));
  return nextOverrides;
}
var transition12 = { bounce: 0.2, delay: 0, duration: 0.4, type: 'spring', };
var Transition2 = ({ value, children, },) => {
  const config = React3.useContext(MotionConfigContext2,);
  const transition = value !== null && value !== void 0 ? value : config.transition;
  const contextValue = React3.useMemo(() => ({ ...config, transition, }), [JSON.stringify(transition,),],);
  return /* @__PURE__ */ _jsx3(MotionConfigContext2.Provider, { value: contextValue, children, },);
};
var Variants2 = motion2.create(React3.Fragment,);
var humanReadableVariantMap2 = { 'Variant 1': 'GnsxM81Tp', 'Variant 2': 'F89hmWHtR', 'Variant 3': 'AwswLpG87', 'Variant 4': 'cNzZOcKSB', };
var getProps2 = ({ height, id, width, ...props },) => {
  var _humanReadableVariantMap_props_variant, _ref;
  return {
    ...props,
    variant:
      (_ref =
            (_humanReadableVariantMap_props_variant = humanReadableVariantMap2[props.variant]) !== null &&
              _humanReadableVariantMap_props_variant !== void 0
              ? _humanReadableVariantMap_props_variant
              : props.variant) !== null && _ref !== void 0
        ? _ref
        : 'GnsxM81Tp',
  };
};
var createLayoutDependency2 = (props, variants,) => {
  if (props.layoutDependency) return variants.join('-',) + props.layoutDependency;
  return variants.join('-',);
};
var Component2 = /* @__PURE__ */ React3.forwardRef(function (props, ref,) {
  const { activeLocale, setLocale, } = useLocaleInfo2();
  const { style, className: className3, layoutId, variant, ...restProps } = getProps2(props,);
  const {
    baseVariant,
    classNames,
    clearLoadingGesture,
    gestureHandlers,
    gestureVariant,
    isLoading,
    setGestureState,
    setVariant,
    variants,
  } = useVariantState2({ cycleOrder: cycleOrder2, defaultVariant: 'GnsxM81Tp', variant, variantClassNames: variantClassNames2, },);
  const layoutDependency = createLayoutDependency2(props, variants,);
  const { activeVariantCallback, delay, } = useActiveVariantCallback2(baseVariant,);
  const onTapwuf733 = activeVariantCallback(async (...args) => {
    setVariant('GnsxM81Tp',);
  },);
  const onTap1kibt4d = activeVariantCallback(async (...args) => {
    setVariant('F89hmWHtR',);
  },);
  const onTap8b8ca0 = activeVariantCallback(async (...args) => {
    setVariant('AwswLpG87',);
  },);
  const onTap19obsyl = activeVariantCallback(async (...args) => {
    setVariant('cNzZOcKSB',);
  },);
  const ref1 = React3.useRef(null,);
  const isDisplayed = () => {
    if (['AwswLpG87', 'cNzZOcKSB',].includes(baseVariant,)) return false;
    return true;
  };
  const isDisplayed1 = () => {
    if (['AwswLpG87', 'cNzZOcKSB',].includes(baseVariant,)) return true;
    return false;
  };
  const defaultLayoutId = React3.useId();
  const sharedStyleClassNames = [];
  const componentViewport = useComponentViewport2();
  return /* @__PURE__ */ _jsx3(LayoutGroup2, {
    id: layoutId !== null && layoutId !== void 0 ? layoutId : defaultLayoutId,
    children: /* @__PURE__ */ _jsx3(Variants2, {
      animate: variants,
      initial: false,
      children: /* @__PURE__ */ _jsx3(Transition2, {
        value: transition12,
        children: /* @__PURE__ */ _jsxs2(motion2.div, {
          ...restProps,
          ...gestureHandlers,
          className: cx2(serializationHash2, ...sharedStyleClassNames, 'framer-1ct4jx9', className3, classNames,),
          'data-framer-name': 'Variant 1',
          layoutDependency,
          layoutId: 'GnsxM81Tp',
          ref: ref !== null && ref !== void 0 ? ref : ref1,
          style: { ...style, },
          ...addPropertyOverrides2(
            {
              AwswLpG87: { 'data-framer-name': 'Variant 3', },
              cNzZOcKSB: { 'data-framer-name': 'Variant 4', },
              F89hmWHtR: { 'data-framer-name': 'Variant 2', },
            },
            baseVariant,
            gestureVariant,
          ),
          children: [
            /* @__PURE__ */ _jsxs2(motion2.div, {
              className: 'framer-13ookua',
              layoutDependency,
              layoutId: 'pSdc0w_WH',
              children: [
                /* @__PURE__ */ _jsx3(RichText2, {
                  __fromCanvasComponent: true,
                  children: /* @__PURE__ */ _jsx3(React3.Fragment, {
                    children: /* @__PURE__ */ _jsx3(motion2.h3, {
                      style: {
                        '--font-selector': 'SW50ZXItQm9sZA==',
                        '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                        '--framer-font-size': '24px',
                        '--framer-font-weight': '700',
                        '--framer-letter-spacing': '-1.9px',
                        '--framer-text-alignment': 'left',
                        '--framer-text-color': 'var(--extracted-a0htzi, rgb(255, 255, 255))',
                      },
                      children: 'On-Demand Hardware',
                    },),
                  },),
                  className: 'framer-ip3pwc',
                  fonts: ['Inter-Bold',],
                  layoutDependency,
                  layoutId: 'qVOJsjIwo',
                  style: {
                    '--extracted-a0htzi': 'rgb(255, 255, 255)',
                    '--framer-link-text-color': 'rgb(0, 153, 255)',
                    '--framer-link-text-decoration': 'underline',
                    '--framer-paragraph-spacing': '0px',
                  },
                  verticalAlignment: 'top',
                  withExternalLayout: true,
                },),
                /* @__PURE__ */ _jsx3(RichText2, {
                  __fromCanvasComponent: true,
                  children: /* @__PURE__ */ _jsx3(React3.Fragment, {
                    children: /* @__PURE__ */ _jsx3(motion2.p, {
                      style: { '--framer-text-alignment': 'left', '--framer-text-color': 'var(--extracted-r6o4lv, rgb(138, 138, 138))', },
                      children: 'For flexible applications that have fluctuations in demand.',
                    },),
                  },),
                  className: 'framer-458qfh',
                  fonts: ['Inter',],
                  layoutDependency,
                  layoutId: 'eetJFznCL',
                  style: { '--extracted-r6o4lv': 'rgb(138, 138, 138)', },
                  verticalAlignment: 'top',
                  withExternalLayout: true,
                },),
              ],
            },),
            /* @__PURE__ */ _jsxs2(motion2.div, {
              className: 'framer-19ho0ab',
              layoutDependency,
              layoutId: 'BC1QzOPfW',
              children: [
                /* @__PURE__ */ _jsx3(motion2.div, {
                  className: 'framer-120oksd',
                  'data-border': true,
                  'data-highlight': true,
                  layoutDependency,
                  layoutId: 'hZInsOYgX',
                  onTap: onTapwuf733,
                  style: {
                    '--border-bottom-width': '1px',
                    '--border-color': 'var(--token-f94bc001-f1ab-463c-abc3-37fb7e541046, rgb(255, 79, 1))',
                    '--border-left-width': '1px',
                    '--border-right-width': '1px',
                    '--border-style': 'solid',
                    '--border-top-width': '1px',
                    borderBottomLeftRadius: 3,
                    borderBottomRightRadius: 3,
                    borderTopLeftRadius: 3,
                    borderTopRightRadius: 3,
                  },
                  variants: {
                    AwswLpG87: {
                      '--border-bottom-width': '0px',
                      '--border-left-width': '0px',
                      '--border-right-width': '0px',
                      '--border-top-width': '0px',
                    },
                    cNzZOcKSB: {
                      '--border-bottom-width': '0px',
                      '--border-left-width': '0px',
                      '--border-right-width': '0px',
                      '--border-top-width': '0px',
                    },
                    F89hmWHtR: {
                      '--border-bottom-width': '0px',
                      '--border-left-width': '0px',
                      '--border-right-width': '0px',
                      '--border-top-width': '0px',
                    },
                  },
                  children: /* @__PURE__ */ _jsx3(RichText2, {
                    __fromCanvasComponent: true,
                    children: /* @__PURE__ */ _jsx3(React3.Fragment, {
                      children: /* @__PURE__ */ _jsx3(motion2.p, {
                        style: {
                          '--font-selector': 'SW50ZXItQm9sZA==',
                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                          '--framer-font-weight': '700',
                          '--framer-text-alignment': 'center',
                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                        },
                        children: 'Community',
                      },),
                    },),
                    className: 'framer-1fkijl',
                    fonts: ['Inter-Bold',],
                    layoutDependency,
                    layoutId: 'itwIOOQpt',
                    style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                    verticalAlignment: 'center',
                    withExternalLayout: true,
                  },),
                },),
                /* @__PURE__ */ _jsx3(motion2.div, {
                  className: 'framer-1bb770e',
                  'data-highlight': true,
                  layoutDependency,
                  layoutId: 'CNWqnFlmE',
                  onTap: onTap1kibt4d,
                  style: {
                    '--border-bottom-width': '0px',
                    '--border-color': 'rgba(0, 0, 0, 0)',
                    '--border-left-width': '0px',
                    '--border-right-width': '0px',
                    '--border-style': 'solid',
                    '--border-top-width': '0px',
                    borderBottomLeftRadius: 0,
                    borderBottomRightRadius: 0,
                    borderTopLeftRadius: 0,
                    borderTopRightRadius: 0,
                  },
                  variants: {
                    AwswLpG87: { borderBottomLeftRadius: 3, borderBottomRightRadius: 3, borderTopLeftRadius: 3, borderTopRightRadius: 3, },
                    cNzZOcKSB: { borderBottomLeftRadius: 3, borderBottomRightRadius: 3, borderTopLeftRadius: 3, borderTopRightRadius: 3, },
                    F89hmWHtR: {
                      '--border-bottom-width': '1px',
                      '--border-color': 'var(--token-f94bc001-f1ab-463c-abc3-37fb7e541046, rgb(255, 79, 1))',
                      '--border-left-width': '1px',
                      '--border-right-width': '1px',
                      '--border-style': 'solid',
                      '--border-top-width': '1px',
                      borderBottomLeftRadius: 3,
                      borderBottomRightRadius: 3,
                      borderTopLeftRadius: 3,
                      borderTopRightRadius: 3,
                    },
                  },
                  ...addPropertyOverrides2({ F89hmWHtR: { 'data-border': true, }, }, baseVariant, gestureVariant,),
                  children: /* @__PURE__ */ _jsx3(RichText2, {
                    __fromCanvasComponent: true,
                    children: /* @__PURE__ */ _jsx3(React3.Fragment, {
                      children: /* @__PURE__ */ _jsx3(motion2.p, {
                        style: {
                          '--font-selector': 'SW50ZXItQm9sZA==',
                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                          '--framer-font-weight': '700',
                          '--framer-text-alignment': 'center',
                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                        },
                        children: 'Pro',
                      },),
                    },),
                    className: 'framer-1gvn0ji',
                    fonts: ['Inter-Bold',],
                    layoutDependency,
                    layoutId: 'C7uaLpr5u',
                    style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                    verticalAlignment: 'center',
                    withExternalLayout: true,
                  },),
                },),
                /* @__PURE__ */ _jsx3(motion2.div, {
                  className: 'framer-r5whag',
                  'data-highlight': true,
                  layoutDependency,
                  layoutId: 'FY7_e_Q_R',
                  onTap: onTap8b8ca0,
                  style: {
                    '--border-bottom-width': '0px',
                    '--border-color': 'rgba(0, 0, 0, 0)',
                    '--border-left-width': '0px',
                    '--border-right-width': '0px',
                    '--border-style': 'solid',
                    '--border-top-width': '0px',
                    borderBottomLeftRadius: 0,
                    borderBottomRightRadius: 0,
                    borderTopLeftRadius: 0,
                    borderTopRightRadius: 0,
                  },
                  variants: {
                    AwswLpG87: {
                      '--border-bottom-width': '1px',
                      '--border-color': 'var(--token-f94bc001-f1ab-463c-abc3-37fb7e541046, rgb(255, 79, 1))',
                      '--border-left-width': '1px',
                      '--border-right-width': '1px',
                      '--border-style': 'solid',
                      '--border-top-width': '1px',
                      borderBottomLeftRadius: 3,
                      borderBottomRightRadius: 3,
                      borderTopLeftRadius: 3,
                      borderTopRightRadius: 3,
                    },
                    cNzZOcKSB: { borderBottomLeftRadius: 3, borderBottomRightRadius: 3, borderTopLeftRadius: 3, borderTopRightRadius: 3, },
                  },
                  ...addPropertyOverrides2({ AwswLpG87: { 'data-border': true, }, }, baseVariant, gestureVariant,),
                  children: /* @__PURE__ */ _jsx3(RichText2, {
                    __fromCanvasComponent: true,
                    children: /* @__PURE__ */ _jsx3(React3.Fragment, {
                      children: /* @__PURE__ */ _jsx3(motion2.p, {
                        style: {
                          '--font-selector': 'SW50ZXItQm9sZA==',
                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                          '--framer-font-weight': '700',
                          '--framer-text-alignment': 'center',
                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                        },
                        children: 'Team',
                      },),
                    },),
                    className: 'framer-17niigl',
                    fonts: ['Inter-Bold',],
                    layoutDependency,
                    layoutId: 'qGI8zpJYU',
                    style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                    verticalAlignment: 'center',
                    withExternalLayout: true,
                  },),
                },),
                /* @__PURE__ */ _jsx3(motion2.div, {
                  className: 'framer-1kl9gaw',
                  'data-highlight': true,
                  layoutDependency,
                  layoutId: 'mY7JPC9Kq',
                  onTap: onTap19obsyl,
                  style: {
                    '--border-bottom-width': '0px',
                    '--border-color': 'rgba(0, 0, 0, 0)',
                    '--border-left-width': '0px',
                    '--border-right-width': '0px',
                    '--border-style': 'solid',
                    '--border-top-width': '0px',
                  },
                  variants: {
                    cNzZOcKSB: {
                      '--border-bottom-width': '1px',
                      '--border-color': 'var(--token-f94bc001-f1ab-463c-abc3-37fb7e541046, rgb(255, 79, 1))',
                      '--border-left-width': '1px',
                      '--border-right-width': '1px',
                      '--border-style': 'solid',
                      '--border-top-width': '1px',
                    },
                  },
                  ...addPropertyOverrides2({ cNzZOcKSB: { 'data-border': true, }, }, baseVariant, gestureVariant,),
                  children: /* @__PURE__ */ _jsx3(RichText2, {
                    __fromCanvasComponent: true,
                    children: /* @__PURE__ */ _jsx3(React3.Fragment, {
                      children: /* @__PURE__ */ _jsx3(motion2.p, {
                        style: {
                          '--font-selector': 'SW50ZXItQm9sZA==',
                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                          '--framer-font-weight': '700',
                          '--framer-text-alignment': 'center',
                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                        },
                        children: 'Enterprise',
                      },),
                    },),
                    className: 'framer-tuiqgt',
                    fonts: ['Inter-Bold',],
                    layoutDependency,
                    layoutId: 'IXdNzyLhs',
                    style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                    verticalAlignment: 'center',
                    withExternalLayout: true,
                  },),
                },),
              ],
            },),
            /* @__PURE__ */ _jsxs2(motion2.div, {
              className: 'framer-3vp09f',
              'data-border': true,
              layoutDependency,
              layoutId: 'qfJUoygAP',
              style: {
                '--border-bottom-width': '0px',
                '--border-color': 'var(--token-be2928fe-8496-42f6-9733-c37829997236, rgb(33, 33, 33))',
                '--border-left-width': '0px',
                '--border-right-width': '0px',
                '--border-style': 'solid',
                '--border-top-width': '1px',
              },
              children: [
                /* @__PURE__ */ _jsx3(motion2.div, {
                  className: 'framer-1cd7t3k',
                  layoutDependency,
                  layoutId: 'luwJStYQp',
                  style: { borderBottomLeftRadius: 3, borderBottomRightRadius: 3, borderTopLeftRadius: 3, borderTopRightRadius: 3, },
                  children: /* @__PURE__ */ _jsx3(RichText2, {
                    __fromCanvasComponent: true,
                    children: /* @__PURE__ */ _jsx3(React3.Fragment, {
                      children: /* @__PURE__ */ _jsx3(motion2.p, {
                        style: {
                          '--font-selector': 'SW50ZXItQm9sZA==',
                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                          '--framer-font-weight': '700',
                          '--framer-text-alignment': 'left',
                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                        },
                        children: 'Regions',
                      },),
                    },),
                    className: 'framer-hwgzcj',
                    fonts: ['Inter-Bold',],
                    layoutDependency,
                    layoutId: 'z60ewVrWR',
                    style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                    verticalAlignment: 'center',
                    withExternalLayout: true,
                  },),
                },),
                /* @__PURE__ */ _jsx3(motion2.div, {
                  className: 'framer-27ocj5',
                  layoutDependency,
                  layoutId: 'KhDog9Ktl',
                  children: /* @__PURE__ */ _jsx3(motion2.div, {
                    className: 'framer-12uw7gq',
                    layoutDependency,
                    layoutId: 'aGy39LfZd',
                    style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                    children: /* @__PURE__ */ _jsx3(RichText2, {
                      __fromCanvasComponent: true,
                      children: /* @__PURE__ */ _jsx3(React3.Fragment, {
                        children: /* @__PURE__ */ _jsx3(motion2.p, {
                          style: {
                            '--framer-text-alignment': 'center',
                            '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                          },
                          children: '2 Regions',
                        },),
                      },),
                      className: 'framer-uh65ym',
                      fonts: ['Inter',],
                      layoutDependency,
                      layoutId: 'EqubBY1tq',
                      style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                      verticalAlignment: 'top',
                      withExternalLayout: true,
                      ...addPropertyOverrides2(
                        {
                          AwswLpG87: {
                            children: /* @__PURE__ */ _jsx3(React3.Fragment, {
                              children: /* @__PURE__ */ _jsx3(motion2.p, {
                                style: {
                                  '--framer-text-alignment': 'center',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                                },
                                children: '8 Regions',
                              },),
                            },),
                          },
                          cNzZOcKSB: {
                            children: /* @__PURE__ */ _jsx3(React3.Fragment, {
                              children: /* @__PURE__ */ _jsx3(motion2.p, {
                                style: {
                                  '--framer-text-alignment': 'center',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                                },
                                children: 'Custom',
                              },),
                            },),
                          },
                          F89hmWHtR: {
                            children: /* @__PURE__ */ _jsx3(React3.Fragment, {
                              children: /* @__PURE__ */ _jsx3(motion2.p, {
                                style: {
                                  '--framer-text-alignment': 'center',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                                },
                                children: '8 Regions',
                              },),
                            },),
                          },
                        },
                        baseVariant,
                        gestureVariant,
                      ),
                    },),
                  },),
                },),
                /* @__PURE__ */ _jsx3(motion2.div, {
                  className: 'framer-i267wp',
                  layoutDependency,
                  layoutId: 'J5iVK9xf4',
                  children: /* @__PURE__ */ _jsx3(RichText2, {
                    __fromCanvasComponent: true,
                    children: /* @__PURE__ */ _jsxs2(React3.Fragment, {
                      children: [
                        /* @__PURE__ */ _jsx3(motion2.p, {
                          style: {
                            '--font-selector': 'R0Y7SW50ZXItNzAw',
                            '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                            '--framer-font-weight': '700',
                            '--framer-text-alignment': 'left',
                            '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                          },
                          children: 'Data Center',
                        },),
                        /* @__PURE__ */ _jsx3(motion2.p, {
                          style: {
                            '--font-selector': 'R0Y7SW50ZXItNzAw',
                            '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                            '--framer-font-weight': '700',
                            '--framer-text-alignment': 'left',
                            '--framer-text-color': 'var(--extracted-2gxw0f, rgb(255, 255, 255))',
                          },
                          children: 'Failover',
                        },),
                      ],
                    },),
                    className: 'framer-17wb7y4',
                    fonts: ['GF;Inter-700',],
                    layoutDependency,
                    layoutId: 'g7HqFOiBQ',
                    style: { '--extracted-2gxw0f': 'rgb(255, 255, 255)', '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                    verticalAlignment: 'center',
                    withExternalLayout: true,
                  },),
                },),
                /* @__PURE__ */ _jsx3(motion2.div, {
                  className: 'framer-18wmvxn',
                  layoutDependency,
                  layoutId: 'cIG5_iqnU',
                  children: /* @__PURE__ */ _jsxs2(motion2.div, {
                    className: 'framer-12tvbgb',
                    layoutDependency,
                    layoutId: 'SIBzagVxH',
                    style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                    children: [
                      isDisplayed() && /* @__PURE__ */ _jsx3(RichText2, {
                        __fromCanvasComponent: true,
                        children: /* @__PURE__ */ _jsx3(React3.Fragment, {
                          children: /* @__PURE__ */ _jsx3(motion2.p, {
                            style: {
                              '--framer-text-alignment': 'center',
                              '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                            },
                            children: '-',
                          },),
                        },),
                        className: 'framer-1l2w3hw',
                        fonts: ['Inter',],
                        layoutDependency,
                        layoutId: 'c3qPK7G9J',
                        style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                        verticalAlignment: 'top',
                        withExternalLayout: true,
                      },),
                      isDisplayed1() && /* @__PURE__ */ _jsx3(SVG2, {
                        className: 'framer-i6tbsk',
                        'data-framer-name': 'Check',
                        layout: 'position',
                        layoutDependency,
                        layoutId: 'APG2zoKra',
                        opacity: 1,
                        radius: 0,
                        style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                        svg:
                          '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 24 24"><path d="M 12 2.25 C 6.615 2.25 2.25 6.615 2.25 12 C 2.25 17.385 6.615 21.75 12 21.75 C 17.385 21.75 21.75 17.385 21.75 12 C 21.74 6.62 17.38 2.26 12 2.25 Z M 16.641 10.294 L 11.147 15.544 C 11.005 15.677 10.817 15.751 10.622 15.75 C 10.429 15.753 10.244 15.679 10.106 15.544 L 7.359 12.919 C 7.152 12.738 7.06 12.458 7.121 12.189 C 7.181 11.92 7.384 11.706 7.649 11.632 C 7.914 11.557 8.199 11.634 8.391 11.831 L 10.622 13.959 L 15.609 9.206 C 15.912 8.942 16.37 8.964 16.647 9.255 C 16.923 9.547 16.921 10.005 16.641 10.294 Z" fill="rgb(255, 79, 1)"></path></svg>',
                        svgContentId: 11856876933,
                        withExternalLayout: true,
                        ...addPropertyOverrides2(
                          { AwswLpG87: { svgContentId: 10285337634, }, cNzZOcKSB: { svgContentId: 10285337634, }, },
                          baseVariant,
                          gestureVariant,
                        ),
                      },),
                    ],
                  },),
                },),
                /* @__PURE__ */ _jsx3(RichText2, {
                  __fromCanvasComponent: true,
                  children: /* @__PURE__ */ _jsxs2(React3.Fragment, {
                    children: [
                      /* @__PURE__ */ _jsx3(motion2.p, {
                        style: {
                          '--font-selector': 'R0Y7SW50ZXItNzAw',
                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                          '--framer-font-weight': '700',
                          '--framer-text-alignment': 'left',
                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                        },
                        children: 'Automatic',
                      },),
                      /* @__PURE__ */ _jsx3(motion2.p, {
                        style: {
                          '--font-selector': 'R0Y7SW50ZXItNzAw',
                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                          '--framer-font-weight': '700',
                          '--framer-text-alignment': 'left',
                          '--framer-text-color': 'var(--extracted-2gxw0f, rgb(255, 255, 255))',
                        },
                        children: 'SSL',
                      },),
                    ],
                  },),
                  className: 'framer-tgn41c',
                  fonts: ['GF;Inter-700',],
                  layoutDependency,
                  layoutId: 'Sj_xPJPtu',
                  style: { '--extracted-2gxw0f': 'rgb(255, 255, 255)', '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                  verticalAlignment: 'center',
                  withExternalLayout: true,
                },),
                /* @__PURE__ */ _jsxs2(motion2.div, {
                  className: 'framer-1w50oy4',
                  layoutDependency,
                  layoutId: 'fC5Ar2SNy',
                  style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                  children: [
                    isDisplayed() && /* @__PURE__ */ _jsx3(RichText2, {
                      __fromCanvasComponent: true,
                      children: /* @__PURE__ */ _jsx3(React3.Fragment, {
                        children: /* @__PURE__ */ _jsx3(motion2.p, {
                          style: {
                            '--framer-text-alignment': 'center',
                            '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                          },
                          children: '-',
                        },),
                      },),
                      className: 'framer-1zq31z',
                      fonts: ['Inter',],
                      layoutDependency,
                      layoutId: 'cCECIBVko',
                      style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                      verticalAlignment: 'top',
                      withExternalLayout: true,
                    },),
                    isDisplayed1() && /* @__PURE__ */ _jsx3(SVG2, {
                      className: 'framer-qnwgs3',
                      'data-framer-name': 'Check',
                      layout: 'position',
                      layoutDependency,
                      layoutId: 'toXNWrYR4',
                      opacity: 1,
                      radius: 0,
                      style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                      svg:
                        '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 24 24"><path d="M 12 2.25 C 6.615 2.25 2.25 6.615 2.25 12 C 2.25 17.385 6.615 21.75 12 21.75 C 17.385 21.75 21.75 17.385 21.75 12 C 21.74 6.62 17.38 2.26 12 2.25 Z M 16.641 10.294 L 11.147 15.544 C 11.005 15.677 10.817 15.751 10.622 15.75 C 10.429 15.753 10.244 15.679 10.106 15.544 L 7.359 12.919 C 7.152 12.738 7.06 12.458 7.121 12.189 C 7.181 11.92 7.384 11.706 7.649 11.632 C 7.914 11.557 8.199 11.634 8.391 11.831 L 10.622 13.959 L 15.609 9.206 C 15.912 8.942 16.37 8.964 16.647 9.255 C 16.923 9.547 16.921 10.005 16.641 10.294 Z" fill="rgb(255, 79, 1)"></path></svg>',
                      svgContentId: 11856876933,
                      withExternalLayout: true,
                      ...addPropertyOverrides2(
                        { AwswLpG87: { svgContentId: 10285337634, }, cNzZOcKSB: { svgContentId: 10285337634, }, },
                        baseVariant,
                        gestureVariant,
                      ),
                    },),
                  ],
                },),
              ],
            },),
          ],
        },),
      },),
    },),
  },);
},);
var css4 = [
  '@supports (aspect-ratio: 1) { body { --framer-aspect-ratio-supported: auto; } }',
  '.framer-lsPSX.framer-9qwxso, .framer-lsPSX .framer-9qwxso { display: block; }',
  '.framer-lsPSX.framer-1ct4jx9 { align-content: center; align-items: center; display: flex; flex-direction: column; flex-wrap: nowrap; gap: 18px; height: min-content; justify-content: center; overflow: visible; padding: 0px; position: relative; width: 240px; }',
  '.framer-lsPSX .framer-13ookua { align-content: flex-start; align-items: flex-start; display: flex; flex: none; flex-direction: column; flex-wrap: nowrap; gap: 2px; height: min-content; justify-content: center; overflow: hidden; padding: 0px; position: relative; width: 100%; }',
  '.framer-lsPSX .framer-ip3pwc { flex: none; height: auto; overflow: visible; position: relative; white-space: pre; width: auto; }',
  '.framer-lsPSX .framer-458qfh { flex: none; height: auto; position: relative; white-space: pre-wrap; width: 100%; word-break: break-word; word-wrap: break-word; }',
  '.framer-lsPSX .framer-19ho0ab { display: grid; flex: none; gap: 10px; grid-auto-rows: minmax(0, 1fr); grid-template-columns: repeat(2, minmax(50px, 1fr)); grid-template-rows: repeat(2, minmax(0, 1fr)); height: 68px; justify-content: center; overflow: hidden; padding: 0px; position: relative; width: 238px; }',
  '.framer-lsPSX .framer-120oksd, .framer-lsPSX .framer-1bb770e, .framer-lsPSX .framer-r5whag, .framer-lsPSX .framer-1kl9gaw { align-self: center; cursor: pointer; flex: none; height: 100%; justify-self: center; position: relative; width: 100%; }',
  '.framer-lsPSX .framer-1fkijl, .framer-lsPSX .framer-hwgzcj { bottom: 0px; flex: none; height: 100%; left: calc(50.00000000000002% - 100% / 2); position: absolute; white-space: pre-wrap; width: 100%; word-break: break-word; word-wrap: break-word; }',
  '.framer-lsPSX .framer-1gvn0ji, .framer-lsPSX .framer-17niigl, .framer-lsPSX .framer-tuiqgt, .framer-lsPSX .framer-17wb7y4 { flex: none; height: 100%; left: 0px; position: absolute; top: 0px; white-space: pre-wrap; width: 100%; word-break: break-word; word-wrap: break-word; }',
  '.framer-lsPSX .framer-3vp09f { display: grid; flex: none; gap: 10px; grid-auto-rows: minmax(0, 1fr); grid-template-columns: repeat(2, minmax(50px, 1fr)); grid-template-rows: repeat(2, minmax(0, 1fr)); height: 224px; justify-content: center; overflow: hidden; padding: 0px; position: relative; width: 238px; }',
  '.framer-lsPSX .framer-1cd7t3k, .framer-lsPSX .framer-27ocj5, .framer-lsPSX .framer-i267wp, .framer-lsPSX .framer-18wmvxn { align-self: center; flex: none; height: 100%; justify-self: center; position: relative; width: 100%; }',
  '.framer-lsPSX .framer-12uw7gq, .framer-lsPSX .framer-12tvbgb { align-content: center; align-items: center; bottom: 0px; display: flex; flex: none; flex-direction: row; flex-wrap: nowrap; gap: 10px; justify-content: center; left: 0px; padding: 0px; position: absolute; right: 0px; top: 0px; }',
  '.framer-lsPSX .framer-uh65ym, .framer-lsPSX .framer-1l2w3hw, .framer-lsPSX .framer-1zq31z { flex: none; height: auto; position: relative; white-space: pre; width: auto; }',
  '.framer-lsPSX .framer-i6tbsk, .framer-lsPSX .framer-qnwgs3 { flex: none; height: 24px; position: relative; width: 24px; }',
  '.framer-lsPSX .framer-tgn41c { align-self: start; flex: none; height: 100%; justify-self: start; position: relative; white-space: pre-wrap; width: 100%; word-break: break-word; word-wrap: break-word; }',
  '.framer-lsPSX .framer-1w50oy4 { align-content: center; align-items: center; align-self: center; display: flex; flex: none; flex-direction: row; flex-wrap: nowrap; gap: 10px; height: 100%; justify-content: center; justify-self: center; padding: 0px; position: relative; width: 100%; }',
  '@supports (background: -webkit-named-image(i)) and (not (font-palette:dark)) { .framer-lsPSX.framer-1ct4jx9, .framer-lsPSX .framer-13ookua, .framer-lsPSX .framer-12uw7gq, .framer-lsPSX .framer-12tvbgb, .framer-lsPSX .framer-1w50oy4 { gap: 0px; } .framer-lsPSX.framer-1ct4jx9 > * { margin: 0px; margin-bottom: calc(18px / 2); margin-top: calc(18px / 2); } .framer-lsPSX.framer-1ct4jx9 > :first-child, .framer-lsPSX .framer-13ookua > :first-child { margin-top: 0px; } .framer-lsPSX.framer-1ct4jx9 > :last-child, .framer-lsPSX .framer-13ookua > :last-child { margin-bottom: 0px; } .framer-lsPSX .framer-13ookua > * { margin: 0px; margin-bottom: calc(2px / 2); margin-top: calc(2px / 2); } .framer-lsPSX .framer-12uw7gq > *, .framer-lsPSX .framer-12tvbgb > *, .framer-lsPSX .framer-1w50oy4 > * { margin: 0px; margin-left: calc(10px / 2); margin-right: calc(10px / 2); } .framer-lsPSX .framer-12uw7gq > :first-child, .framer-lsPSX .framer-12tvbgb > :first-child, .framer-lsPSX .framer-1w50oy4 > :first-child { margin-left: 0px; } .framer-lsPSX .framer-12uw7gq > :last-child, .framer-lsPSX .framer-12tvbgb > :last-child, .framer-lsPSX .framer-1w50oy4 > :last-child { margin-right: 0px; } }',
  '.framer-lsPSX[data-border="true"]::after, .framer-lsPSX [data-border="true"]::after { content: ""; border-width: var(--border-top-width, 0) var(--border-right-width, 0) var(--border-bottom-width, 0) var(--border-left-width, 0); border-color: var(--border-color, none); border-style: var(--border-style, none); width: 100%; height: 100%; position: absolute; box-sizing: border-box; left: 0; top: 0; border-radius: inherit; pointer-events: none; }',
];
var FramersjRZudCNs = withCSS2(Component2, css4, 'framer-lsPSX',);
var stdin_default2 = FramersjRZudCNs;
FramersjRZudCNs.displayName = 'On-Demand Mobile';
FramersjRZudCNs.defaultProps = { height: 397, width: 240, };
addPropertyControls3(FramersjRZudCNs, {
  variant: {
    options: ['GnsxM81Tp', 'F89hmWHtR', 'AwswLpG87', 'cNzZOcKSB',],
    optionTitles: ['Variant 1', 'Variant 2', 'Variant 3', 'Variant 4',],
    title: 'Variant',
    type: ControlType3.Enum,
  },
},);
addFonts2(FramersjRZudCNs, [{
  explicitInter: true,
  fonts: [{
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0460-052F, U+1C80-1C88, U+20B4, U+2DE0-2DFF, U+A640-A69F, U+FE2E-FE2F',
    url: 'https://framerusercontent.com/assets/DpPBYI0sL4fYLgAkX8KXOPVt7c.woff2',
    weight: '700',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0301, U+0400-045F, U+0490-0491, U+04B0-04B1, U+2116',
    url: 'https://framerusercontent.com/assets/4RAEQdEOrcnDkhHiiCbJOw92Lk.woff2',
    weight: '700',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+1F00-1FFF',
    url: 'https://framerusercontent.com/assets/1K3W8DizY3v4emK8Mb08YHxTbs.woff2',
    weight: '700',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0370-03FF',
    url: 'https://framerusercontent.com/assets/tUSCtfYVM1I1IchuyCwz9gDdQ.woff2',
    weight: '700',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0100-024F, U+0259, U+1E00-1EFF, U+2020, U+20A0-20AB, U+20AD-20CF, U+2113, U+2C60-2C7F, U+A720-A7FF',
    url: 'https://framerusercontent.com/assets/VgYFWiwsAC5OYxAycRXXvhze58.woff2',
    weight: '700',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange:
      'U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+2000-206F, U+2074, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD',
    url: 'https://framerusercontent.com/assets/DXD0Q7LSl7HEvDzucnyLnGBHM.woff2',
    weight: '700',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0102-0103, U+0110-0111, U+0128-0129, U+0168-0169, U+01A0-01A1, U+01AF-01B0, U+1EA0-1EF9, U+20AB',
    url: 'https://framerusercontent.com/assets/GIryZETIX4IFypco5pYZONKhJIo.woff2',
    weight: '700',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0460-052F, U+1C80-1C88, U+20B4, U+2DE0-2DFF, U+A640-A69F, U+FE2E-FE2F',
    url: 'https://framerusercontent.com/assets/5vvr9Vy74if2I6bQbJvbw7SY1pQ.woff2',
    weight: '400',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0301, U+0400-045F, U+0490-0491, U+04B0-04B1, U+2116',
    url: 'https://framerusercontent.com/assets/EOr0mi4hNtlgWNn9if640EZzXCo.woff2',
    weight: '400',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+1F00-1FFF',
    url: 'https://framerusercontent.com/assets/Y9k9QrlZAqio88Klkmbd8VoMQc.woff2',
    weight: '400',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0370-03FF',
    url: 'https://framerusercontent.com/assets/OYrD2tBIBPvoJXiIHnLoOXnY9M.woff2',
    weight: '400',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0100-024F, U+0259, U+1E00-1EFF, U+2020, U+20A0-20AB, U+20AD-20CF, U+2113, U+2C60-2C7F, U+A720-A7FF',
    url: 'https://framerusercontent.com/assets/JeYwfuaPfZHQhEG8U5gtPDZ7WQ.woff2',
    weight: '400',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange:
      'U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+2000-206F, U+2074, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD',
    url: 'https://framerusercontent.com/assets/vQyevYAyHtARFwPqUzQGpnDs.woff2',
    weight: '400',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0102-0103, U+0110-0111, U+0128-0129, U+0168-0169, U+01A0-01A1, U+01AF-01B0, U+1EA0-1EF9, U+20AB',
    url: 'https://framerusercontent.com/assets/b6Y37FthZeALduNqHicBT6FutY.woff2',
    weight: '400',
  }, {
    family: 'Inter',
    source: 'google',
    style: 'normal',
    url: 'https://fonts.gstatic.com/s/inter/v18/UcCO3FwrK3iLTeHuS_nVMrMxCp50SjIw2boKoduKmMEVuFuYMZ1rib2Bg-4.woff2',
    weight: '700',
  },],
},], { supportsExplicitInterCodegen: true, },);

// https :https://framerusercontent.com/modules/49xYYGOFO93Skde5TFCH/YOUXfYa7N6OU5ElYboWj/X1cUiBHGB.js
import { jsx as _jsx4, jsxs as _jsxs3, } from 'react/jsx-runtime';
import {
  addFonts as addFonts3,
  addPropertyControls as addPropertyControls4,
  ControlType as ControlType4,
  cx as cx3,
  RichText as RichText3,
  useActiveVariantCallback as useActiveVariantCallback3,
  useComponentViewport as useComponentViewport3,
  useLocaleInfo as useLocaleInfo3,
  useVariantState as useVariantState3,
  withCSS as withCSS3,
} from 'unframer';
import { LayoutGroup as LayoutGroup3, motion as motion3, MotionConfigContext as MotionConfigContext3, } from 'unframer';
import * as React4 from 'react';
var cycleOrder3 = ['Op3GPHO8w', 'aSjpvI_cG', 'ECznCyn5G', 'OevHXiKyH',];
var serializationHash3 = 'framer-JXqMD';
var variantClassNames3 = {
  aSjpvI_cG: 'framer-v-1b5x57o',
  ECznCyn5G: 'framer-v-13oozvi',
  OevHXiKyH: 'framer-v-4en605',
  Op3GPHO8w: 'framer-v-toalvb',
};
function addPropertyOverrides3(overrides, ...variants) {
  const nextOverrides = {};
  variants === null || variants === void 0
    ? void 0
    : variants.forEach((variant,) => variant && Object.assign(nextOverrides, overrides[variant],));
  return nextOverrides;
}
var transition13 = { bounce: 0.2, delay: 0, duration: 0.4, type: 'spring', };
var Transition3 = ({ value, children, },) => {
  const config = React4.useContext(MotionConfigContext3,);
  const transition = value !== null && value !== void 0 ? value : config.transition;
  const contextValue = React4.useMemo(() => ({ ...config, transition, }), [JSON.stringify(transition,),],);
  return /* @__PURE__ */ _jsx4(MotionConfigContext3.Provider, { value: contextValue, children, },);
};
var Variants3 = motion3.create(React4.Fragment,);
var humanReadableVariantMap3 = { 'Variant 1': 'Op3GPHO8w', 'Variant 2': 'aSjpvI_cG', 'Variant 3': 'ECznCyn5G', 'Variant 4': 'OevHXiKyH', };
var getProps3 = ({ height, id, width, ...props },) => {
  var _humanReadableVariantMap_props_variant, _ref;
  return {
    ...props,
    variant:
      (_ref =
            (_humanReadableVariantMap_props_variant = humanReadableVariantMap3[props.variant]) !== null &&
              _humanReadableVariantMap_props_variant !== void 0
              ? _humanReadableVariantMap_props_variant
              : props.variant) !== null && _ref !== void 0
        ? _ref
        : 'Op3GPHO8w',
  };
};
var createLayoutDependency3 = (props, variants,) => {
  if (props.layoutDependency) return variants.join('-',) + props.layoutDependency;
  return variants.join('-',);
};
var Component3 = /* @__PURE__ */ React4.forwardRef(function (props, ref,) {
  const { activeLocale, setLocale, } = useLocaleInfo3();
  const { style, className: className3, layoutId, variant, ...restProps } = getProps3(props,);
  const {
    baseVariant,
    classNames,
    clearLoadingGesture,
    gestureHandlers,
    gestureVariant,
    isLoading,
    setGestureState,
    setVariant,
    variants,
  } = useVariantState3({ cycleOrder: cycleOrder3, defaultVariant: 'Op3GPHO8w', variant, variantClassNames: variantClassNames3, },);
  const layoutDependency = createLayoutDependency3(props, variants,);
  const { activeVariantCallback, delay, } = useActiveVariantCallback3(baseVariant,);
  const onTap178bolc = activeVariantCallback(async (...args) => {
    setVariant('Op3GPHO8w',);
  },);
  const onTap1m74hkv = activeVariantCallback(async (...args) => {
    setVariant('aSjpvI_cG',);
  },);
  const onTap1dcskhy = activeVariantCallback(async (...args) => {
    setVariant('ECznCyn5G',);
  },);
  const onTap2pr56c = activeVariantCallback(async (...args) => {
    setVariant('OevHXiKyH',);
  },);
  const ref1 = React4.useRef(null,);
  const defaultLayoutId = React4.useId();
  const sharedStyleClassNames = [];
  const componentViewport = useComponentViewport3();
  return /* @__PURE__ */ _jsx4(LayoutGroup3, {
    id: layoutId !== null && layoutId !== void 0 ? layoutId : defaultLayoutId,
    children: /* @__PURE__ */ _jsx4(Variants3, {
      animate: variants,
      initial: false,
      children: /* @__PURE__ */ _jsx4(Transition3, {
        value: transition13,
        children: /* @__PURE__ */ _jsxs3(motion3.div, {
          ...restProps,
          ...gestureHandlers,
          className: cx3(serializationHash3, ...sharedStyleClassNames, 'framer-toalvb', className3, classNames,),
          'data-framer-name': 'Variant 1',
          layoutDependency,
          layoutId: 'Op3GPHO8w',
          ref: ref !== null && ref !== void 0 ? ref : ref1,
          style: { ...style, },
          ...addPropertyOverrides3(
            {
              aSjpvI_cG: { 'data-framer-name': 'Variant 2', },
              ECznCyn5G: { 'data-framer-name': 'Variant 3', },
              OevHXiKyH: { 'data-framer-name': 'Variant 4', },
            },
            baseVariant,
            gestureVariant,
          ),
          children: [
            /* @__PURE__ */ _jsxs3(motion3.div, {
              className: 'framer-1d0teu3',
              layoutDependency,
              layoutId: 'gngzTflBk',
              children: [
                /* @__PURE__ */ _jsx4(RichText3, {
                  __fromCanvasComponent: true,
                  children: /* @__PURE__ */ _jsx4(React4.Fragment, {
                    children: /* @__PURE__ */ _jsx4(motion3.h3, {
                      style: {
                        '--font-selector': 'SW50ZXItQm9sZA==',
                        '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                        '--framer-font-size': '24px',
                        '--framer-font-weight': '700',
                        '--framer-letter-spacing': '-1.9px',
                        '--framer-text-alignment': 'left',
                        '--framer-text-color': 'var(--extracted-a0htzi, rgb(255, 255, 255))',
                      },
                      children: 'Plan Features',
                    },),
                  },),
                  className: 'framer-m8busg',
                  fonts: ['Inter-Bold',],
                  layoutDependency,
                  layoutId: 'J9s8HiihH',
                  style: {
                    '--extracted-a0htzi': 'rgb(255, 255, 255)',
                    '--framer-link-text-color': 'rgb(0, 153, 255)',
                    '--framer-link-text-decoration': 'underline',
                    '--framer-paragraph-spacing': '0px',
                  },
                  verticalAlignment: 'top',
                  withExternalLayout: true,
                },),
                /* @__PURE__ */ _jsx4(RichText3, {
                  __fromCanvasComponent: true,
                  children: /* @__PURE__ */ _jsx4(React4.Fragment, {
                    children: /* @__PURE__ */ _jsx4(motion3.p, {
                      style: { '--framer-text-alignment': 'left', '--framer-text-color': 'var(--extracted-r6o4lv, rgb(138, 138, 138))', },
                      children: 'Start for free with scalable pricing as you grow.',
                    },),
                  },),
                  className: 'framer-mrabgx',
                  fonts: ['Inter',],
                  layoutDependency,
                  layoutId: 'tQExtaUpx',
                  style: { '--extracted-r6o4lv': 'rgb(138, 138, 138)', },
                  verticalAlignment: 'top',
                  withExternalLayout: true,
                },),
              ],
            },),
            /* @__PURE__ */ _jsxs3(motion3.div, {
              className: 'framer-6bh7oi',
              layoutDependency,
              layoutId: 'OOlLVHfzh',
              children: [
                /* @__PURE__ */ _jsx4(motion3.div, {
                  className: 'framer-qe9cvk',
                  'data-border': true,
                  layoutDependency,
                  layoutId: 'XKLaEPSGg',
                  style: {
                    '--border-bottom-width': '1px',
                    '--border-color': 'var(--token-f94bc001-f1ab-463c-abc3-37fb7e541046, rgb(255, 79, 1))',
                    '--border-left-width': '1px',
                    '--border-right-width': '1px',
                    '--border-style': 'solid',
                    '--border-top-width': '1px',
                    borderBottomLeftRadius: 3,
                    borderBottomRightRadius: 3,
                    borderTopLeftRadius: 3,
                    borderTopRightRadius: 3,
                  },
                  variants: {
                    aSjpvI_cG: {
                      '--border-bottom-width': '0px',
                      '--border-left-width': '0px',
                      '--border-right-width': '0px',
                      '--border-top-width': '0px',
                    },
                    ECznCyn5G: {
                      '--border-bottom-width': '0px',
                      '--border-left-width': '0px',
                      '--border-right-width': '0px',
                      '--border-top-width': '0px',
                    },
                    OevHXiKyH: {
                      '--border-bottom-width': '0px',
                      '--border-left-width': '0px',
                      '--border-right-width': '0px',
                      '--border-top-width': '0px',
                    },
                  },
                  ...addPropertyOverrides3(
                    {
                      aSjpvI_cG: { 'data-highlight': true, onTap: onTap178bolc, },
                      ECznCyn5G: { 'data-highlight': true, onTap: onTap178bolc, },
                      OevHXiKyH: { 'data-highlight': true, onTap: onTap178bolc, },
                    },
                    baseVariant,
                    gestureVariant,
                  ),
                  children: /* @__PURE__ */ _jsx4(RichText3, {
                    __fromCanvasComponent: true,
                    children: /* @__PURE__ */ _jsx4(React4.Fragment, {
                      children: /* @__PURE__ */ _jsx4(motion3.p, {
                        style: {
                          '--font-selector': 'SW50ZXItQm9sZA==',
                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                          '--framer-font-weight': '700',
                          '--framer-text-alignment': 'center',
                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                        },
                        children: 'Community',
                      },),
                    },),
                    className: 'framer-1u8lg85',
                    'data-highlight': true,
                    fonts: ['Inter-Bold',],
                    layoutDependency,
                    layoutId: 'QrSXfRB7L',
                    onTap: onTap178bolc,
                    style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                    verticalAlignment: 'center',
                    withExternalLayout: true,
                    ...addPropertyOverrides3(
                      { ECznCyn5G: { 'data-highlight': void 0, onTap: void 0, }, OevHXiKyH: { 'data-highlight': void 0, onTap: void 0, }, },
                      baseVariant,
                      gestureVariant,
                    ),
                  },),
                },),
                /* @__PURE__ */ _jsx4(motion3.div, {
                  className: 'framer-6em7qz',
                  'data-highlight': true,
                  layoutDependency,
                  layoutId: 'PHEkRt4I5',
                  onTap: onTap1m74hkv,
                  style: {
                    '--border-bottom-width': '0px',
                    '--border-color': 'rgba(0, 0, 0, 0)',
                    '--border-left-width': '0px',
                    '--border-right-width': '0px',
                    '--border-style': 'solid',
                    '--border-top-width': '0px',
                    borderBottomLeftRadius: 0,
                    borderBottomRightRadius: 0,
                    borderTopLeftRadius: 0,
                    borderTopRightRadius: 0,
                  },
                  variants: {
                    aSjpvI_cG: {
                      '--border-bottom-width': '1px',
                      '--border-color': 'var(--token-f94bc001-f1ab-463c-abc3-37fb7e541046, rgb(255, 79, 1))',
                      '--border-left-width': '1px',
                      '--border-right-width': '1px',
                      '--border-style': 'solid',
                      '--border-top-width': '1px',
                      borderBottomLeftRadius: 3,
                      borderBottomRightRadius: 3,
                      borderTopLeftRadius: 3,
                      borderTopRightRadius: 3,
                    },
                    ECznCyn5G: { borderBottomLeftRadius: 3, borderBottomRightRadius: 3, borderTopLeftRadius: 3, borderTopRightRadius: 3, },
                    OevHXiKyH: { borderBottomLeftRadius: 3, borderBottomRightRadius: 3, borderTopLeftRadius: 3, borderTopRightRadius: 3, },
                  },
                  ...addPropertyOverrides3({ aSjpvI_cG: { 'data-border': true, }, }, baseVariant, gestureVariant,),
                  children: /* @__PURE__ */ _jsx4(RichText3, {
                    __fromCanvasComponent: true,
                    children: /* @__PURE__ */ _jsx4(React4.Fragment, {
                      children: /* @__PURE__ */ _jsx4(motion3.p, {
                        style: {
                          '--font-selector': 'SW50ZXItQm9sZA==',
                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                          '--framer-font-weight': '700',
                          '--framer-text-alignment': 'center',
                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                        },
                        children: 'Pro',
                      },),
                    },),
                    className: 'framer-1vgz6jt',
                    fonts: ['Inter-Bold',],
                    layoutDependency,
                    layoutId: 'RVsi94eOW',
                    style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                    verticalAlignment: 'center',
                    withExternalLayout: true,
                  },),
                },),
                /* @__PURE__ */ _jsx4(motion3.div, {
                  className: 'framer-3srkg2',
                  'data-highlight': true,
                  layoutDependency,
                  layoutId: 'nI0fGAWz6',
                  onTap: onTap1dcskhy,
                  style: {
                    '--border-bottom-width': '0px',
                    '--border-color': 'rgba(0, 0, 0, 0)',
                    '--border-left-width': '0px',
                    '--border-right-width': '0px',
                    '--border-style': 'solid',
                    '--border-top-width': '0px',
                    borderBottomLeftRadius: 0,
                    borderBottomRightRadius: 0,
                    borderTopLeftRadius: 0,
                    borderTopRightRadius: 0,
                  },
                  variants: {
                    ECznCyn5G: {
                      '--border-bottom-width': '1px',
                      '--border-color': 'var(--token-f94bc001-f1ab-463c-abc3-37fb7e541046, rgb(255, 79, 1))',
                      '--border-left-width': '1px',
                      '--border-right-width': '1px',
                      '--border-style': 'solid',
                      '--border-top-width': '1px',
                      borderBottomLeftRadius: 3,
                      borderBottomRightRadius: 3,
                      borderTopLeftRadius: 3,
                      borderTopRightRadius: 3,
                    },
                    OevHXiKyH: { borderBottomLeftRadius: 3, borderBottomRightRadius: 3, borderTopLeftRadius: 3, borderTopRightRadius: 3, },
                  },
                  ...addPropertyOverrides3({ ECznCyn5G: { 'data-border': true, }, }, baseVariant, gestureVariant,),
                  children: /* @__PURE__ */ _jsx4(RichText3, {
                    __fromCanvasComponent: true,
                    children: /* @__PURE__ */ _jsx4(React4.Fragment, {
                      children: /* @__PURE__ */ _jsx4(motion3.p, {
                        style: {
                          '--font-selector': 'SW50ZXItQm9sZA==',
                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                          '--framer-font-weight': '700',
                          '--framer-text-alignment': 'center',
                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                        },
                        children: 'Team',
                      },),
                    },),
                    className: 'framer-1yf749n',
                    fonts: ['Inter-Bold',],
                    layoutDependency,
                    layoutId: 'Uo70LrcKH',
                    style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                    verticalAlignment: 'center',
                    withExternalLayout: true,
                  },),
                },),
                /* @__PURE__ */ _jsx4(motion3.div, {
                  className: 'framer-5deehd',
                  'data-highlight': true,
                  layoutDependency,
                  layoutId: 'r53yTaZHn',
                  onTap: onTap2pr56c,
                  style: {
                    '--border-bottom-width': '0px',
                    '--border-color': 'rgba(0, 0, 0, 0)',
                    '--border-left-width': '0px',
                    '--border-right-width': '0px',
                    '--border-style': 'solid',
                    '--border-top-width': '0px',
                    borderBottomLeftRadius: 0,
                    borderBottomRightRadius: 0,
                    borderTopLeftRadius: 0,
                    borderTopRightRadius: 0,
                  },
                  variants: {
                    OevHXiKyH: {
                      '--border-bottom-width': '1px',
                      '--border-color': 'var(--token-f94bc001-f1ab-463c-abc3-37fb7e541046, rgb(255, 79, 1))',
                      '--border-left-width': '1px',
                      '--border-right-width': '1px',
                      '--border-style': 'solid',
                      '--border-top-width': '1px',
                      borderBottomLeftRadius: 3,
                      borderBottomRightRadius: 3,
                      borderTopLeftRadius: 3,
                      borderTopRightRadius: 3,
                    },
                  },
                  ...addPropertyOverrides3({ OevHXiKyH: { 'data-border': true, }, }, baseVariant, gestureVariant,),
                  children: /* @__PURE__ */ _jsx4(RichText3, {
                    __fromCanvasComponent: true,
                    children: /* @__PURE__ */ _jsx4(React4.Fragment, {
                      children: /* @__PURE__ */ _jsx4(motion3.p, {
                        style: {
                          '--font-selector': 'SW50ZXItQm9sZA==',
                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                          '--framer-font-weight': '700',
                          '--framer-text-alignment': 'center',
                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                        },
                        children: 'Enterprise',
                      },),
                    },),
                    className: 'framer-8emkvd',
                    fonts: ['Inter-Bold',],
                    layoutDependency,
                    layoutId: 'La_3ozs9b',
                    style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                    verticalAlignment: 'center',
                    withExternalLayout: true,
                  },),
                },),
              ],
            },),
            /* @__PURE__ */ _jsxs3(motion3.div, {
              className: 'framer-fyiu02',
              'data-border': true,
              layoutDependency,
              layoutId: 'wAtSd8mt1',
              style: {
                '--border-bottom-width': '0px',
                '--border-color': 'var(--token-be2928fe-8496-42f6-9733-c37829997236, rgb(33, 33, 33))',
                '--border-left-width': '0px',
                '--border-right-width': '0px',
                '--border-style': 'solid',
                '--border-top-width': '1px',
              },
              children: [
                /* @__PURE__ */ _jsx4(motion3.div, {
                  className: 'framer-bahzll',
                  layoutDependency,
                  layoutId: 'b_OiJqkcJ',
                  style: { borderBottomLeftRadius: 3, borderBottomRightRadius: 3, borderTopLeftRadius: 3, borderTopRightRadius: 3, },
                  children: /* @__PURE__ */ _jsx4(RichText3, {
                    __fromCanvasComponent: true,
                    children: /* @__PURE__ */ _jsx4(React4.Fragment, {
                      children: /* @__PURE__ */ _jsx4(motion3.p, {
                        style: {
                          '--font-selector': 'SW50ZXItQm9sZA==',
                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                          '--framer-font-weight': '700',
                          '--framer-text-alignment': 'left',
                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                        },
                        children: 'Log Retention',
                      },),
                    },),
                    className: 'framer-oixmb9',
                    fonts: ['Inter-Bold',],
                    layoutDependency,
                    layoutId: 'gai03ftM2',
                    style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                    verticalAlignment: 'center',
                    withExternalLayout: true,
                  },),
                },),
                /* @__PURE__ */ _jsx4(motion3.div, {
                  className: 'framer-m9daqf',
                  layoutDependency,
                  layoutId: 'LLimOdCiK',
                  children: /* @__PURE__ */ _jsx4(motion3.div, {
                    className: 'framer-fcrody',
                    layoutDependency,
                    layoutId: 'rT2ajiNuy',
                    style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                    children: /* @__PURE__ */ _jsx4(RichText3, {
                      __fromCanvasComponent: true,
                      children: /* @__PURE__ */ _jsx4(React4.Fragment, {
                        children: /* @__PURE__ */ _jsx4(motion3.p, {
                          style: {
                            '--framer-text-alignment': 'center',
                            '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                          },
                          children: '24 Hours',
                        },),
                      },),
                      className: 'framer-1q62h1',
                      fonts: ['Inter',],
                      layoutDependency,
                      layoutId: 'EfdYNmhyY',
                      style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                      verticalAlignment: 'top',
                      withExternalLayout: true,
                      ...addPropertyOverrides3(
                        {
                          aSjpvI_cG: {
                            children: /* @__PURE__ */ _jsx4(React4.Fragment, {
                              children: /* @__PURE__ */ _jsx4(motion3.p, {
                                style: {
                                  '--framer-text-alignment': 'center',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                                },
                                children: '14 Days',
                              },),
                            },),
                          },
                          ECznCyn5G: {
                            children: /* @__PURE__ */ _jsx4(React4.Fragment, {
                              children: /* @__PURE__ */ _jsx4(motion3.p, {
                                style: {
                                  '--framer-text-alignment': 'center',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                                },
                                children: '30 Days',
                              },),
                            },),
                          },
                          OevHXiKyH: {
                            children: /* @__PURE__ */ _jsx4(React4.Fragment, {
                              children: /* @__PURE__ */ _jsx4(motion3.p, {
                                style: {
                                  '--framer-text-alignment': 'center',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                                },
                                children: '6 Months',
                              },),
                            },),
                          },
                        },
                        baseVariant,
                        gestureVariant,
                      ),
                    },),
                  },),
                },),
                /* @__PURE__ */ _jsx4(motion3.div, {
                  className: 'framer-ueq6hi',
                  layoutDependency,
                  layoutId: 'PsVSyXCLw',
                  children: /* @__PURE__ */ _jsx4(RichText3, {
                    __fromCanvasComponent: true,
                    children: /* @__PURE__ */ _jsxs3(React4.Fragment, {
                      children: [
                        /* @__PURE__ */ _jsx4(motion3.p, {
                          style: {
                            '--font-selector': 'R0Y7SW50ZXItNzAw',
                            '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                            '--framer-font-weight': '700',
                            '--framer-text-alignment': 'left',
                            '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                          },
                          children: 'Build ',
                        },),
                        /* @__PURE__ */ _jsx4(motion3.p, {
                          style: {
                            '--font-selector': 'R0Y7SW50ZXItNzAw',
                            '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                            '--framer-font-weight': '700',
                            '--framer-text-alignment': 'left',
                            '--framer-text-color': 'var(--extracted-2gxw0f, rgb(255, 255, 255))',
                          },
                          children: 'Retention',
                        },),
                      ],
                    },),
                    className: 'framer-dtazg8',
                    fonts: ['GF;Inter-700',],
                    layoutDependency,
                    layoutId: 'rFYFT5gHE',
                    style: { '--extracted-2gxw0f': 'rgb(255, 255, 255)', '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                    verticalAlignment: 'center',
                    withExternalLayout: true,
                  },),
                },),
                /* @__PURE__ */ _jsx4(motion3.div, {
                  className: 'framer-1lcgrsx',
                  layoutDependency,
                  layoutId: 'TR3VBSXLP',
                  children: /* @__PURE__ */ _jsx4(motion3.div, {
                    className: 'framer-kll52c',
                    layoutDependency,
                    layoutId: 'KQAOsPdDb',
                    style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                    children: /* @__PURE__ */ _jsx4(RichText3, {
                      __fromCanvasComponent: true,
                      children: /* @__PURE__ */ _jsx4(React4.Fragment, {
                        children: /* @__PURE__ */ _jsx4(motion3.p, {
                          style: {
                            '--framer-text-alignment': 'center',
                            '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                          },
                          children: '24 Hours',
                        },),
                      },),
                      className: 'framer-14umh1h',
                      fonts: ['Inter',],
                      layoutDependency,
                      layoutId: 'dTRHY2Zqe',
                      style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                      verticalAlignment: 'top',
                      withExternalLayout: true,
                      ...addPropertyOverrides3(
                        {
                          aSjpvI_cG: {
                            children: /* @__PURE__ */ _jsx4(React4.Fragment, {
                              children: /* @__PURE__ */ _jsx4(motion3.p, {
                                style: {
                                  '--framer-text-alignment': 'center',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                                },
                                children: '14 Days',
                              },),
                            },),
                          },
                          ECznCyn5G: {
                            children: /* @__PURE__ */ _jsx4(React4.Fragment, {
                              children: /* @__PURE__ */ _jsx4(motion3.p, {
                                style: {
                                  '--framer-text-alignment': 'center',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                                },
                                children: '30 Days',
                              },),
                            },),
                          },
                          OevHXiKyH: {
                            children: /* @__PURE__ */ _jsx4(React4.Fragment, {
                              children: /* @__PURE__ */ _jsx4(motion3.p, {
                                style: {
                                  '--framer-text-alignment': 'center',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                                },
                                children: '6 Months',
                              },),
                            },),
                          },
                        },
                        baseVariant,
                        gestureVariant,
                      ),
                    },),
                  },),
                },),
                /* @__PURE__ */ _jsx4(RichText3, {
                  __fromCanvasComponent: true,
                  children: /* @__PURE__ */ _jsx4(React4.Fragment, {
                    children: /* @__PURE__ */ _jsx4(motion3.p, {
                      style: {
                        '--font-selector': 'R0Y7SW50ZXItNzAw',
                        '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                        '--framer-font-weight': '700',
                        '--framer-text-alignment': 'left',
                        '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                      },
                      children: 'Support',
                    },),
                  },),
                  className: 'framer-1vtf3hs',
                  fonts: ['GF;Inter-700',],
                  layoutDependency,
                  layoutId: 'Ygt4AkFhZ',
                  style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                  verticalAlignment: 'center',
                  withExternalLayout: true,
                },),
                /* @__PURE__ */ _jsx4(motion3.div, {
                  className: 'framer-14xgsk0',
                  layoutDependency,
                  layoutId: 'Ab8diZrq4',
                  style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                  children: /* @__PURE__ */ _jsx4(RichText3, {
                    __fromCanvasComponent: true,
                    children: /* @__PURE__ */ _jsxs3(React4.Fragment, {
                      children: [
                        /* @__PURE__ */ _jsx4(motion3.p, {
                          style: {
                            '--framer-text-alignment': 'center',
                            '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                          },
                          children: 'Community',
                        },),
                        /* @__PURE__ */ _jsx4(motion3.p, {
                          style: {
                            '--framer-text-alignment': 'center',
                            '--framer-text-color': 'var(--extracted-2gxw0f, rgb(136, 136, 136))',
                          },
                          children: 'Support',
                        },),
                      ],
                    },),
                    className: 'framer-uai94a',
                    fonts: ['Inter',],
                    layoutDependency,
                    layoutId: 'R8iFtfU91',
                    style: { '--extracted-2gxw0f': 'rgb(136, 136, 136)', '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                    verticalAlignment: 'top',
                    withExternalLayout: true,
                    ...addPropertyOverrides3(
                      {
                        aSjpvI_cG: {
                          children: /* @__PURE__ */ _jsx4(React4.Fragment, {
                            children: /* @__PURE__ */ _jsx4(motion3.p, {
                              style: {
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                              },
                              children: 'Email',
                            },),
                          },),
                        },
                        ECznCyn5G: {
                          children: /* @__PURE__ */ _jsxs3(React4.Fragment, {
                            children: [
                              /* @__PURE__ */ _jsx4(motion3.p, {
                                style: {
                                  '--framer-text-alignment': 'center',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                                },
                                children: 'Priority ',
                              },),
                              /* @__PURE__ */ _jsx4(motion3.p, {
                                style: {
                                  '--framer-text-alignment': 'center',
                                  '--framer-text-color': 'var(--extracted-2gxw0f, rgb(136, 136, 136))',
                                },
                                children: 'Email',
                              },),
                            ],
                          },),
                        },
                        OevHXiKyH: {
                          children: /* @__PURE__ */ _jsxs3(React4.Fragment, {
                            children: [
                              /* @__PURE__ */ _jsx4(motion3.p, {
                                style: {
                                  '--framer-text-alignment': 'center',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                                },
                                children: 'Chat and ',
                              },),
                              /* @__PURE__ */ _jsx4(motion3.p, {
                                style: {
                                  '--framer-text-alignment': 'center',
                                  '--framer-text-color': 'var(--extracted-2gxw0f, rgb(136, 136, 136))',
                                },
                                children: 'Priority Email',
                              },),
                            ],
                          },),
                        },
                      },
                      baseVariant,
                      gestureVariant,
                    ),
                  },),
                },),
              ],
            },),
          ],
        },),
      },),
    },),
  },);
},);
var css5 = [
  '@supports (aspect-ratio: 1) { body { --framer-aspect-ratio-supported: auto; } }',
  '.framer-JXqMD.framer-8rfgz9, .framer-JXqMD .framer-8rfgz9 { display: block; }',
  '.framer-JXqMD.framer-toalvb { align-content: center; align-items: center; display: flex; flex-direction: column; flex-wrap: nowrap; gap: 18px; height: min-content; justify-content: center; overflow: visible; padding: 0px; position: relative; width: 240px; }',
  '.framer-JXqMD .framer-1d0teu3 { align-content: flex-start; align-items: flex-start; display: flex; flex: none; flex-direction: column; flex-wrap: nowrap; gap: 2px; height: min-content; justify-content: center; overflow: hidden; padding: 0px; position: relative; width: 100%; }',
  '.framer-JXqMD .framer-m8busg { flex: none; height: auto; overflow: visible; position: relative; white-space: pre; width: auto; }',
  '.framer-JXqMD .framer-mrabgx { flex: none; height: auto; position: relative; white-space: pre-wrap; width: 100%; word-break: break-word; word-wrap: break-word; }',
  '.framer-JXqMD .framer-6bh7oi { display: grid; flex: none; gap: 10px; grid-auto-rows: minmax(0, 1fr); grid-template-columns: repeat(2, minmax(50px, 1fr)); grid-template-rows: repeat(2, minmax(0, 1fr)); height: 68px; justify-content: center; overflow: hidden; padding: 0px; position: relative; width: 238px; }',
  '.framer-JXqMD .framer-qe9cvk, .framer-JXqMD .framer-bahzll, .framer-JXqMD .framer-m9daqf, .framer-JXqMD .framer-ueq6hi, .framer-JXqMD .framer-1lcgrsx { align-self: center; flex: none; height: 100%; justify-self: center; position: relative; width: 100%; }',
  '.framer-JXqMD .framer-1u8lg85 { bottom: 0px; cursor: pointer; flex: none; height: 100%; left: calc(50.00000000000002% - 100% / 2); position: absolute; white-space: pre-wrap; width: 100%; word-break: break-word; word-wrap: break-word; }',
  '.framer-JXqMD .framer-6em7qz, .framer-JXqMD .framer-3srkg2, .framer-JXqMD .framer-5deehd { align-self: center; cursor: pointer; flex: none; height: 100%; justify-self: center; position: relative; width: 100%; }',
  '.framer-JXqMD .framer-1vgz6jt, .framer-JXqMD .framer-1yf749n, .framer-JXqMD .framer-8emkvd, .framer-JXqMD .framer-dtazg8 { flex: none; height: 100%; left: 0px; position: absolute; top: 0px; white-space: pre-wrap; width: 100%; word-break: break-word; word-wrap: break-word; }',
  '.framer-JXqMD .framer-fyiu02 { display: grid; flex: none; gap: 10px; grid-auto-rows: minmax(0, 1fr); grid-template-columns: repeat(2, minmax(50px, 1fr)); grid-template-rows: repeat(2, minmax(0, 1fr)); height: 224px; justify-content: center; overflow: hidden; padding: 0px; position: relative; width: 238px; }',
  '.framer-JXqMD .framer-oixmb9 { bottom: 0px; flex: none; height: 100%; left: calc(50.00000000000002% - 100% / 2); position: absolute; white-space: pre-wrap; width: 100%; word-break: break-word; word-wrap: break-word; }',
  '.framer-JXqMD .framer-fcrody, .framer-JXqMD .framer-kll52c { align-content: center; align-items: center; bottom: 0px; display: flex; flex: none; flex-direction: row; flex-wrap: nowrap; gap: 10px; justify-content: center; left: 0px; padding: 0px; position: absolute; right: 0px; top: 0px; }',
  '.framer-JXqMD .framer-1q62h1, .framer-JXqMD .framer-14umh1h, .framer-JXqMD .framer-uai94a { flex: none; height: auto; position: relative; white-space: pre; width: auto; }',
  '.framer-JXqMD .framer-1vtf3hs { align-self: start; flex: none; height: 100%; justify-self: start; position: relative; white-space: pre-wrap; width: 100%; word-break: break-word; word-wrap: break-word; }',
  '.framer-JXqMD .framer-14xgsk0 { align-content: center; align-items: center; align-self: center; display: flex; flex: none; flex-direction: row; flex-wrap: nowrap; gap: 10px; height: 100%; justify-content: center; justify-self: center; padding: 0px; position: relative; width: 100%; }',
  '@supports (background: -webkit-named-image(i)) and (not (font-palette:dark)) { .framer-JXqMD.framer-toalvb, .framer-JXqMD .framer-1d0teu3, .framer-JXqMD .framer-fcrody, .framer-JXqMD .framer-kll52c, .framer-JXqMD .framer-14xgsk0 { gap: 0px; } .framer-JXqMD.framer-toalvb > * { margin: 0px; margin-bottom: calc(18px / 2); margin-top: calc(18px / 2); } .framer-JXqMD.framer-toalvb > :first-child, .framer-JXqMD .framer-1d0teu3 > :first-child { margin-top: 0px; } .framer-JXqMD.framer-toalvb > :last-child, .framer-JXqMD .framer-1d0teu3 > :last-child { margin-bottom: 0px; } .framer-JXqMD .framer-1d0teu3 > * { margin: 0px; margin-bottom: calc(2px / 2); margin-top: calc(2px / 2); } .framer-JXqMD .framer-fcrody > *, .framer-JXqMD .framer-kll52c > *, .framer-JXqMD .framer-14xgsk0 > * { margin: 0px; margin-left: calc(10px / 2); margin-right: calc(10px / 2); } .framer-JXqMD .framer-fcrody > :first-child, .framer-JXqMD .framer-kll52c > :first-child, .framer-JXqMD .framer-14xgsk0 > :first-child { margin-left: 0px; } .framer-JXqMD .framer-fcrody > :last-child, .framer-JXqMD .framer-kll52c > :last-child, .framer-JXqMD .framer-14xgsk0 > :last-child { margin-right: 0px; } }',
  '.framer-JXqMD.framer-v-1b5x57o .framer-qe9cvk, .framer-JXqMD.framer-v-13oozvi .framer-qe9cvk, .framer-JXqMD.framer-v-4en605 .framer-qe9cvk { cursor: pointer; }',
  '.framer-JXqMD.framer-v-13oozvi .framer-1u8lg85, .framer-JXqMD.framer-v-4en605 .framer-1u8lg85 { cursor: unset; }',
  '.framer-JXqMD[data-border="true"]::after, .framer-JXqMD [data-border="true"]::after { content: ""; border-width: var(--border-top-width, 0) var(--border-right-width, 0) var(--border-bottom-width, 0) var(--border-left-width, 0); border-color: var(--border-color, none); border-style: var(--border-style, none); width: 100%; height: 100%; position: absolute; box-sizing: border-box; left: 0; top: 0; border-radius: inherit; pointer-events: none; }',
];
var FramerX1cUiBHGB = withCSS3(Component3, css5, 'framer-JXqMD',);
var stdin_default3 = FramerX1cUiBHGB;
FramerX1cUiBHGB.displayName = 'Mobile Plan Features';
FramerX1cUiBHGB.defaultProps = { height: 397, width: 240, };
addPropertyControls4(FramerX1cUiBHGB, {
  variant: {
    options: ['Op3GPHO8w', 'aSjpvI_cG', 'ECznCyn5G', 'OevHXiKyH',],
    optionTitles: ['Variant 1', 'Variant 2', 'Variant 3', 'Variant 4',],
    title: 'Variant',
    type: ControlType4.Enum,
  },
},);
addFonts3(FramerX1cUiBHGB, [{
  explicitInter: true,
  fonts: [{
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0460-052F, U+1C80-1C88, U+20B4, U+2DE0-2DFF, U+A640-A69F, U+FE2E-FE2F',
    url: 'https://framerusercontent.com/assets/DpPBYI0sL4fYLgAkX8KXOPVt7c.woff2',
    weight: '700',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0301, U+0400-045F, U+0490-0491, U+04B0-04B1, U+2116',
    url: 'https://framerusercontent.com/assets/4RAEQdEOrcnDkhHiiCbJOw92Lk.woff2',
    weight: '700',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+1F00-1FFF',
    url: 'https://framerusercontent.com/assets/1K3W8DizY3v4emK8Mb08YHxTbs.woff2',
    weight: '700',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0370-03FF',
    url: 'https://framerusercontent.com/assets/tUSCtfYVM1I1IchuyCwz9gDdQ.woff2',
    weight: '700',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0100-024F, U+0259, U+1E00-1EFF, U+2020, U+20A0-20AB, U+20AD-20CF, U+2113, U+2C60-2C7F, U+A720-A7FF',
    url: 'https://framerusercontent.com/assets/VgYFWiwsAC5OYxAycRXXvhze58.woff2',
    weight: '700',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange:
      'U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+2000-206F, U+2074, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD',
    url: 'https://framerusercontent.com/assets/DXD0Q7LSl7HEvDzucnyLnGBHM.woff2',
    weight: '700',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0102-0103, U+0110-0111, U+0128-0129, U+0168-0169, U+01A0-01A1, U+01AF-01B0, U+1EA0-1EF9, U+20AB',
    url: 'https://framerusercontent.com/assets/GIryZETIX4IFypco5pYZONKhJIo.woff2',
    weight: '700',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0460-052F, U+1C80-1C88, U+20B4, U+2DE0-2DFF, U+A640-A69F, U+FE2E-FE2F',
    url: 'https://framerusercontent.com/assets/5vvr9Vy74if2I6bQbJvbw7SY1pQ.woff2',
    weight: '400',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0301, U+0400-045F, U+0490-0491, U+04B0-04B1, U+2116',
    url: 'https://framerusercontent.com/assets/EOr0mi4hNtlgWNn9if640EZzXCo.woff2',
    weight: '400',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+1F00-1FFF',
    url: 'https://framerusercontent.com/assets/Y9k9QrlZAqio88Klkmbd8VoMQc.woff2',
    weight: '400',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0370-03FF',
    url: 'https://framerusercontent.com/assets/OYrD2tBIBPvoJXiIHnLoOXnY9M.woff2',
    weight: '400',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0100-024F, U+0259, U+1E00-1EFF, U+2020, U+20A0-20AB, U+20AD-20CF, U+2113, U+2C60-2C7F, U+A720-A7FF',
    url: 'https://framerusercontent.com/assets/JeYwfuaPfZHQhEG8U5gtPDZ7WQ.woff2',
    weight: '400',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange:
      'U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+2000-206F, U+2074, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD',
    url: 'https://framerusercontent.com/assets/vQyevYAyHtARFwPqUzQGpnDs.woff2',
    weight: '400',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0102-0103, U+0110-0111, U+0128-0129, U+0168-0169, U+01A0-01A1, U+01AF-01B0, U+1EA0-1EF9, U+20AB',
    url: 'https://framerusercontent.com/assets/b6Y37FthZeALduNqHicBT6FutY.woff2',
    weight: '400',
  }, {
    family: 'Inter',
    source: 'google',
    style: 'normal',
    url: 'https://fonts.gstatic.com/s/inter/v18/UcCO3FwrK3iLTeHuS_nVMrMxCp50SjIw2boKoduKmMEVuFuYMZ1rib2Bg-4.woff2',
    weight: '700',
  },],
},], { supportsExplicitInterCodegen: true, },);

// https :https://framerusercontent.com/modules/EQc0WVLWSNg3MefFYC3W/q1ZVKhr4UZwXUATr4xTE/kTXE7wUBN.js
var FeatherFonts = getFonts(Icon,);
var MobilePlanFeaturesFonts = getFonts(stdin_default3,);
var OnDemandMobileFonts = getFonts(stdin_default2,);
var DedicatedMobileFonts = getFonts(stdin_default,);
var cycleOrder4 = ['bAm7TcIeo', 'wm4kyLmlr', 'OlQQ934Vt',];
var serializationHash4 = 'framer-i47EY';
var variantClassNames4 = { bAm7TcIeo: 'framer-v-7simsb', OlQQ934Vt: 'framer-v-14tpty9', wm4kyLmlr: 'framer-v-3f72b1', };
function addPropertyOverrides4(overrides, ...variants) {
  const nextOverrides = {};
  variants === null || variants === void 0
    ? void 0
    : variants.forEach((variant,) => variant && Object.assign(nextOverrides, overrides[variant],));
  return nextOverrides;
}
var transition14 = { bounce: 0.2, delay: 0, duration: 0.4, type: 'spring', };
var Transition4 = ({ value, children, },) => {
  const config = React5.useContext(MotionConfigContext4,);
  const transition = value !== null && value !== void 0 ? value : config.transition;
  const contextValue = React5.useMemo(() => ({ ...config, transition, }), [JSON.stringify(transition,),],);
  return /* @__PURE__ */ _jsx5(MotionConfigContext4.Provider, { value: contextValue, children, },);
};
var Variants4 = motion4.create(React5.Fragment,);
var humanReadableVariantMap4 = { Desktop: 'bAm7TcIeo', phone: 'OlQQ934Vt', Tablet: 'wm4kyLmlr', };
var getProps4 = ({ height, id, width, ...props },) => {
  var _humanReadableVariantMap_props_variant, _ref;
  return {
    ...props,
    variant:
      (_ref =
            (_humanReadableVariantMap_props_variant = humanReadableVariantMap4[props.variant]) !== null &&
              _humanReadableVariantMap_props_variant !== void 0
              ? _humanReadableVariantMap_props_variant
              : props.variant) !== null && _ref !== void 0
        ? _ref
        : 'bAm7TcIeo',
  };
};
var createLayoutDependency4 = (props, variants,) => {
  if (props.layoutDependency) return variants.join('-',) + props.layoutDependency;
  return variants.join('-',);
};
var Component4 = /* @__PURE__ */ React5.forwardRef(function (props, ref,) {
  const { activeLocale, setLocale, } = useLocaleInfo4();
  const { style, className: className3, layoutId, variant, ...restProps } = getProps4(props,);
  const {
    baseVariant,
    classNames,
    clearLoadingGesture,
    gestureHandlers,
    gestureVariant,
    isLoading,
    setGestureState,
    setVariant,
    variants,
  } = useVariantState4({ cycleOrder: cycleOrder4, defaultVariant: 'bAm7TcIeo', variant, variantClassNames: variantClassNames4, },);
  const layoutDependency = createLayoutDependency4(props, variants,);
  const ref1 = React5.useRef(null,);
  const isDisplayed = () => {
    if (baseVariant === 'OlQQ934Vt') return false;
    return true;
  };
  const isDisplayed1 = () => {
    if (baseVariant === 'OlQQ934Vt') return true;
    return false;
  };
  const defaultLayoutId = React5.useId();
  const sharedStyleClassNames = [className2, className,];
  const componentViewport = useComponentViewport4();
  return /* @__PURE__ */ _jsx5(LayoutGroup4, {
    id: layoutId !== null && layoutId !== void 0 ? layoutId : defaultLayoutId,
    children: /* @__PURE__ */ _jsx5(Variants4, {
      animate: variants,
      initial: false,
      children: /* @__PURE__ */ _jsx5(Transition4, {
        value: transition14,
        children: /* @__PURE__ */ _jsxs4(motion4.section, {
          ...restProps,
          ...gestureHandlers,
          className: cx4(serializationHash4, ...sharedStyleClassNames, 'framer-7simsb', className3, classNames,),
          'data-framer-name': 'Desktop',
          layoutDependency,
          layoutId: 'bAm7TcIeo',
          ref: ref !== null && ref !== void 0 ? ref : ref1,
          style: { backgroundColor: 'rgb(0, 0, 0)', ...style, },
          ...addPropertyOverrides4(
            { OlQQ934Vt: { 'data-framer-name': 'phone', }, wm4kyLmlr: { 'data-framer-name': 'Tablet', }, },
            baseVariant,
            gestureVariant,
          ),
          children: [
            isDisplayed() && /* @__PURE__ */ _jsxs4(motion4.div, {
              className: 'framer-q4z8b',
              layoutDependency,
              layoutId: 'k_2Wa5JKu',
              children: [
                /* @__PURE__ */ _jsx5(RichText4, {
                  __fromCanvasComponent: true,
                  children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                    children: /* @__PURE__ */ _jsx5(motion4.h3, {
                      className: 'framer-styles-preset-jttjmp',
                      'data-styles-preset': 'zu841OiIg',
                      style: { '--framer-text-alignment': 'center', },
                      children: 'Rivet Cloud Pricing',
                    },),
                  },),
                  className: 'framer-yel58z',
                  fonts: ['Inter',],
                  layoutDependency,
                  layoutId: 'bEryo798l',
                  style: {
                    '--framer-link-text-color': 'rgb(0, 153, 255)',
                    '--framer-link-text-decoration': 'underline',
                    '--framer-paragraph-spacing': '0px',
                  },
                  verticalAlignment: 'top',
                  withExternalLayout: true,
                },),
                /* @__PURE__ */ _jsx5(RichText4, {
                  __fromCanvasComponent: true,
                  children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                    children: /* @__PURE__ */ _jsx5(motion4.h2, {
                      className: 'framer-styles-preset-az499w',
                      'data-styles-preset': 'kHb0JRZSH',
                      style: { '--framer-text-alignment': 'center', },
                      children: 'Start for free with scalable pricing as you grow.',
                    },),
                  },),
                  className: 'framer-58hm9z',
                  fonts: ['Inter',],
                  layoutDependency,
                  layoutId: 'DClsNZALg',
                  style: { '--framer-paragraph-spacing': '0px', },
                  verticalAlignment: 'top',
                  withExternalLayout: true,
                },),
              ],
            },),
            isDisplayed() && /* @__PURE__ */ _jsx5(motion4.div, {
              className: 'framer-1voyydo',
              layoutDependency,
              layoutId: 'mhRKORo3Z',
              children: /* @__PURE__ */ _jsxs4(motion4.div, {
                className: 'framer-lbd5da',
                'data-framer-name': 'Container',
                layoutDependency,
                layoutId: 'V6xr2fXCA',
                children: [
                  /* @__PURE__ */ _jsxs4(motion4.div, {
                    className: 'framer-1y83tpy',
                    'data-border': true,
                    'data-framer-name': 'Card',
                    layoutDependency,
                    layoutId: 'NBOTxS4FR',
                    style: {
                      '--border-bottom-width': '1px',
                      '--border-color': 'rgb(77, 77, 77)',
                      '--border-left-width': '1px',
                      '--border-right-width': '1px',
                      '--border-style': 'solid',
                      '--border-top-width': '1px',
                      backgroundColor: 'rgb(8, 8, 8)',
                      borderBottomLeftRadius: 6,
                      borderBottomRightRadius: 6,
                      borderTopLeftRadius: 6,
                      borderTopRightRadius: 6,
                      boxShadow:
                        '0px 0.7961918735236395px 2.3885756205709185px -0.625px rgba(0, 0, 0, 0.05), 0px 2.414506143104518px 7.2435184293135535px -1.25px rgba(0, 0, 0, 0.05), 0px 6.382653521484461px 19.147960564453385px -1.875px rgba(0, 0, 0, 0.05), 0px 20px 60px -2.5px rgba(0, 0, 0, 0.05)',
                    },
                    children: [
                      /* @__PURE__ */ _jsx5(RichText4, {
                        __fromCanvasComponent: true,
                        children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                          children: /* @__PURE__ */ _jsx5(motion4.p, {
                            style: {
                              '--font-selector': 'R0Y7SW50ZXItNjAw',
                              '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                              '--framer-font-size': '24px',
                              '--framer-font-weight': '600',
                              '--framer-line-height': '1.5em',
                              '--framer-text-alignment': 'left',
                              '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                            },
                            children: 'Community',
                          },),
                        },),
                        className: 'framer-1lgaaot',
                        fonts: ['GF;Inter-600',],
                        layoutDependency,
                        layoutId: 'g35Q6wXBK',
                        style: {
                          '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                          '--framer-link-text-color': 'rgb(0, 153, 255)',
                          '--framer-link-text-decoration': 'underline',
                          '--framer-paragraph-spacing': '0px',
                        },
                        verticalAlignment: 'top',
                        withExternalLayout: true,
                      },),
                      /* @__PURE__ */ _jsxs4(motion4.div, {
                        className: 'framer-h9t7w7',
                        layoutDependency,
                        layoutId: 'zfhsRTIAA',
                        children: [
                          /* @__PURE__ */ _jsx5(RichText4, {
                            __fromCanvasComponent: true,
                            children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                              children: /* @__PURE__ */ _jsx5(motion4.p, {
                                style: {
                                  '--font-selector': 'R0Y7SW50ZXItNzAw',
                                  '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                  '--framer-font-size': '40px',
                                  '--framer-font-weight': '700',
                                  '--framer-letter-spacing': '-3px',
                                  '--framer-line-height': '1em',
                                  '--framer-text-alignment': 'left',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                                },
                                children: '$0',
                              },),
                            },),
                            className: 'framer-2hibes',
                            fonts: ['GF;Inter-700',],
                            layoutDependency,
                            layoutId: 'ibKZHXppW',
                            style: {
                              '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                              '--framer-link-text-color': 'rgb(0, 153, 255)',
                              '--framer-link-text-decoration': 'underline',
                              '--framer-paragraph-spacing': '0px',
                            },
                            verticalAlignment: 'top',
                            withExternalLayout: true,
                          },),
                          /* @__PURE__ */ _jsx5(RichText4, {
                            __fromCanvasComponent: true,
                            children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                              children: /* @__PURE__ */ _jsx5(motion4.p, {
                                style: {
                                  '--font-selector': 'R0Y7SW50ZXItNjAw',
                                  '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                  '--framer-font-size': '14px',
                                  '--framer-font-weight': '600',
                                  '--framer-line-height': '1.5em',
                                  '--framer-text-alignment': 'left',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                },
                                children: '/month',
                              },),
                            },),
                            className: 'framer-11gi34',
                            fonts: ['GF;Inter-600',],
                            layoutDependency,
                            layoutId: 'FtK2qZDbB',
                            style: {
                              '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                              '--framer-link-text-color': 'rgb(0, 153, 255)',
                              '--framer-link-text-decoration': 'underline',
                              '--framer-paragraph-spacing': '0px',
                            },
                            verticalAlignment: 'top',
                            withExternalLayout: true,
                          },),
                        ],
                      },),
                      /* @__PURE__ */ _jsx5(RichText4, {
                        __fromCanvasComponent: true,
                        children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                          children: /* @__PURE__ */ _jsx5(motion4.p, {
                            style: {
                              '--font-selector': 'R0Y7SW50ZXItNjAw',
                              '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                              '--framer-font-size': '14px',
                              '--framer-font-weight': '600',
                              '--framer-line-height': '1.5em',
                              '--framer-text-alignment': 'left',
                              '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                            },
                            children: '+Resource Usage',
                          },),
                        },),
                        className: 'framer-136ha75',
                        fonts: ['GF;Inter-600',],
                        layoutDependency,
                        layoutId: 'wRFVPkh6W',
                        style: {
                          '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                          '--framer-link-text-color': 'rgb(0, 153, 255)',
                          '--framer-link-text-decoration': 'underline',
                          '--framer-paragraph-spacing': '0px',
                        },
                        verticalAlignment: 'top',
                        withExternalLayout: true,
                      },),
                      /* @__PURE__ */ _jsxs4(motion4.div, {
                        className: 'framer-5yabb0',
                        layoutDependency,
                        layoutId: 'u3mgHxPPV',
                        children: [
                          /* @__PURE__ */ _jsxs4(motion4.div, {
                            className: 'framer-axiu75',
                            'data-framer-name': 'Row',
                            layoutDependency,
                            layoutId: 'Vst0pyl0T',
                            children: [
                              /* @__PURE__ */ _jsx5(ComponentViewportProvider, {
                                children: /* @__PURE__ */ _jsx5(motion4.div, {
                                  className: 'framer-7hjajm-container',
                                  layoutDependency,
                                  layoutId: 'Q6tTWnblh-container',
                                  children: /* @__PURE__ */ _jsx5(Icon, {
                                    color: 'rgb(255, 255, 255)',
                                    height: '100%',
                                    iconSearch: 'Home',
                                    iconSelection: 'gift',
                                    id: 'Q6tTWnblh',
                                    layoutId: 'Q6tTWnblh',
                                    mirrored: false,
                                    selectByList: true,
                                    style: { height: '100%', width: '100%', },
                                    width: '100%',
                                  },),
                                },),
                              },),
                              /* @__PURE__ */ _jsx5(RichText4, {
                                __fromCanvasComponent: true,
                                children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                  children: /* @__PURE__ */ _jsx5(motion4.p, {
                                    style: {
                                      '--font-selector': 'R0Y7SW50ZXItNjAw',
                                      '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                      '--framer-font-size': '12px',
                                      '--framer-font-weight': '600',
                                      '--framer-line-height': '1.5em',
                                      '--framer-text-alignment': 'left',
                                      '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                    },
                                    children: '$5 monthly credit',
                                  },),
                                },),
                                className: 'framer-1tyoeu8',
                                fonts: ['GF;Inter-600',],
                                layoutDependency,
                                layoutId: 'iZEWSxl6O',
                                style: {
                                  '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                  '--framer-link-text-color': 'rgb(0, 153, 255)',
                                  '--framer-link-text-decoration': 'underline',
                                  '--framer-paragraph-spacing': '0px',
                                },
                                verticalAlignment: 'top',
                                withExternalLayout: true,
                              },),
                            ],
                          },),
                          /* @__PURE__ */ _jsxs4(motion4.div, {
                            className: 'framer-q5ntcw',
                            'data-framer-name': 'Row',
                            layoutDependency,
                            layoutId: 'GfHQFeks8',
                            children: [
                              /* @__PURE__ */ _jsx5(SVG3, {
                                className: 'framer-8v83xe',
                                'data-framer-name': 'Check',
                                layout: 'position',
                                layoutDependency,
                                layoutId: 'y5F0agD8V',
                                opacity: 1,
                                radius: 0,
                                style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                svg:
                                  '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                svgContentId: 10075247493,
                                withExternalLayout: true,
                              },),
                              /* @__PURE__ */ _jsx5(RichText4, {
                                __fromCanvasComponent: true,
                                children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                  children: /* @__PURE__ */ _jsx5(motion4.p, {
                                    style: {
                                      '--font-selector': 'R0Y7SW50ZXItNjAw',
                                      '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                      '--framer-font-size': '12px',
                                      '--framer-font-weight': '600',
                                      '--framer-line-height': '1.5em',
                                      '--framer-text-alignment': 'left',
                                      '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                    },
                                    children: 'DDoS Mitigation',
                                  },),
                                },),
                                className: 'framer-1wz2c6l',
                                fonts: ['GF;Inter-600',],
                                layoutDependency,
                                layoutId: 'I1dWiN8FK',
                                style: {
                                  '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                  '--framer-link-text-color': 'rgb(0, 153, 255)',
                                  '--framer-link-text-decoration': 'underline',
                                  '--framer-paragraph-spacing': '0px',
                                },
                                verticalAlignment: 'top',
                                withExternalLayout: true,
                              },),
                            ],
                          },),
                          /* @__PURE__ */ _jsxs4(motion4.div, {
                            className: 'framer-1hb3cwx',
                            'data-framer-name': 'Row',
                            layoutDependency,
                            layoutId: 'TNOVBS23e',
                            children: [
                              /* @__PURE__ */ _jsx5(SVG3, {
                                className: 'framer-14onrej',
                                'data-framer-name': 'Check',
                                layout: 'position',
                                layoutDependency,
                                layoutId: 'Jl7obl6aB',
                                opacity: 1,
                                radius: 0,
                                style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                svg:
                                  '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                svgContentId: 10075247493,
                                withExternalLayout: true,
                              },),
                              /* @__PURE__ */ _jsx5(RichText4, {
                                __fromCanvasComponent: true,
                                children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                  children: /* @__PURE__ */ _jsx5(motion4.p, {
                                    style: {
                                      '--font-selector': 'R0Y7SW50ZXItNjAw',
                                      '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                      '--framer-font-size': '12px',
                                      '--framer-font-weight': '600',
                                      '--framer-line-height': '1.5em',
                                      '--framer-text-alignment': 'left',
                                      '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                    },
                                    children: 'SSL Management ',
                                  },),
                                },),
                                className: 'framer-l7re06',
                                fonts: ['GF;Inter-600',],
                                layoutDependency,
                                layoutId: 'AiXWTagep',
                                style: {
                                  '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                  '--framer-link-text-color': 'rgb(0, 153, 255)',
                                  '--framer-link-text-decoration': 'underline',
                                  '--framer-paragraph-spacing': '0px',
                                },
                                verticalAlignment: 'top',
                                withExternalLayout: true,
                              },),
                            ],
                          },),
                          /* @__PURE__ */ _jsxs4(motion4.div, {
                            className: 'framer-z2gxu',
                            'data-framer-name': 'Row',
                            layoutDependency,
                            layoutId: 'aBTa7M0yk',
                            children: [
                              /* @__PURE__ */ _jsx5(SVG3, {
                                className: 'framer-1c7wwes',
                                'data-framer-name': 'Check',
                                layout: 'position',
                                layoutDependency,
                                layoutId: 'pcdt722vm',
                                opacity: 1,
                                radius: 0,
                                style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                svg:
                                  '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                svgContentId: 10075247493,
                                withExternalLayout: true,
                              },),
                              /* @__PURE__ */ _jsx5(RichText4, {
                                __fromCanvasComponent: true,
                                children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                  children: /* @__PURE__ */ _jsx5(motion4.p, {
                                    style: {
                                      '--font-selector': 'R0Y7SW50ZXItNjAw',
                                      '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                      '--framer-font-size': '12px',
                                      '--framer-font-weight': '600',
                                      '--framer-line-height': '1.5em',
                                      '--framer-text-alignment': 'left',
                                      '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                    },
                                    children: 'Community Support',
                                  },),
                                },),
                                className: 'framer-1nzr4ba',
                                fonts: ['GF;Inter-600',],
                                layoutDependency,
                                layoutId: 'Dd62xWXNz',
                                style: {
                                  '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                  '--framer-link-text-color': 'rgb(0, 153, 255)',
                                  '--framer-link-text-decoration': 'underline',
                                  '--framer-paragraph-spacing': '0px',
                                },
                                verticalAlignment: 'top',
                                withExternalLayout: true,
                              },),
                            ],
                          },),
                        ],
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1vsbw7d',
                        'data-framer-name': 'Button',
                        layoutDependency,
                        layoutId: 'FZuasu6EX',
                        style: {
                          backgroundColor: 'rgb(255, 79, 0)',
                          borderBottomLeftRadius: 4,
                          borderBottomRightRadius: 4,
                          borderTopLeftRadius: 4,
                          borderTopRightRadius: 4,
                          boxShadow:
                            '0px 0.7065919983928324px 0.7065919983928324px -0.625px rgba(0, 0, 0, 0.14764), 0px 1.8065619053231785px 1.8065619053231785px -1.25px rgba(0, 0, 0, 0.14398), 0px 3.6217592146567767px 3.6217592146567767px -1.875px rgba(0, 0, 0, 0.13793), 0px 6.8655999097303715px 6.8655999097303715px -2.5px rgba(0, 0, 0, 0.12711), 0px 13.646761411524492px 13.646761411524492px -3.125px rgba(0, 0, 0, 0.10451), 0px 30px 30px -3.75px rgba(0, 0, 0, 0.05)',
                        },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--font-selector': 'R0Y7SW50ZXItNjAw',
                                '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                '--framer-font-size': '14px',
                                '--framer-font-weight': '600',
                                '--framer-letter-spacing': '0px',
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                              },
                              children: 'Get Started',
                            },),
                          },),
                          className: 'framer-kocp2l',
                          fonts: ['GF;Inter-600',],
                          layoutDependency,
                          layoutId: 'jDZbqBV2T',
                          style: {
                            '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                            '--framer-link-text-color': 'rgb(0, 153, 255)',
                            '--framer-link-text-decoration': 'underline',
                            '--framer-paragraph-spacing': '0px',
                          },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                    ],
                  },),
                  /* @__PURE__ */ _jsxs4(motion4.div, {
                    className: 'framer-1p01z0t',
                    'data-border': true,
                    'data-framer-name': 'Card',
                    layoutDependency,
                    layoutId: 'VIVEeru92',
                    style: {
                      '--border-bottom-width': '1px',
                      '--border-color': 'rgb(77, 77, 77)',
                      '--border-left-width': '1px',
                      '--border-right-width': '1px',
                      '--border-style': 'solid',
                      '--border-top-width': '1px',
                      backgroundColor: 'rgb(8, 8, 8)',
                      borderBottomLeftRadius: 6,
                      borderBottomRightRadius: 6,
                      borderTopLeftRadius: 6,
                      borderTopRightRadius: 6,
                      boxShadow:
                        '0px 0.7961918735236395px 2.3885756205709185px -0.625px rgba(0, 0, 0, 0.05), 0px 2.414506143104518px 7.2435184293135535px -1.25px rgba(0, 0, 0, 0.05), 0px 6.382653521484461px 19.147960564453385px -1.875px rgba(0, 0, 0, 0.05), 0px 20px 60px -2.5px rgba(0, 0, 0, 0.05)',
                    },
                    children: [
                      /* @__PURE__ */ _jsx5(RichText4, {
                        __fromCanvasComponent: true,
                        children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                          children: /* @__PURE__ */ _jsx5(motion4.p, {
                            style: {
                              '--font-selector': 'R0Y7SW50ZXItNjAw',
                              '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                              '--framer-font-size': '24px',
                              '--framer-font-weight': '600',
                              '--framer-line-height': '1.5em',
                              '--framer-text-alignment': 'left',
                              '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                            },
                            children: 'Pro',
                          },),
                        },),
                        className: 'framer-5rkukc',
                        fonts: ['GF;Inter-600',],
                        layoutDependency,
                        layoutId: 'EzQDoxPAN',
                        style: {
                          '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                          '--framer-link-text-color': 'rgb(0, 153, 255)',
                          '--framer-link-text-decoration': 'underline',
                          '--framer-paragraph-spacing': '0px',
                        },
                        verticalAlignment: 'top',
                        withExternalLayout: true,
                      },),
                      /* @__PURE__ */ _jsxs4(motion4.div, {
                        className: 'framer-nm4aj0',
                        layoutDependency,
                        layoutId: 'lsMnuZ6LB',
                        children: [
                          /* @__PURE__ */ _jsx5(RichText4, {
                            __fromCanvasComponent: true,
                            children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                              children: /* @__PURE__ */ _jsx5(motion4.p, {
                                style: {
                                  '--font-selector': 'R0Y7SW50ZXItNzAw',
                                  '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                  '--framer-font-size': '40px',
                                  '--framer-font-weight': '700',
                                  '--framer-letter-spacing': '-3px',
                                  '--framer-line-height': '1em',
                                  '--framer-text-alignment': 'left',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                                },
                                children: '$20',
                              },),
                            },),
                            className: 'framer-1oe7fs6',
                            fonts: ['GF;Inter-700',],
                            layoutDependency,
                            layoutId: 'IsFLbJ_gj',
                            style: {
                              '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                              '--framer-link-text-color': 'rgb(0, 153, 255)',
                              '--framer-link-text-decoration': 'underline',
                              '--framer-paragraph-spacing': '0px',
                            },
                            verticalAlignment: 'top',
                            withExternalLayout: true,
                          },),
                          /* @__PURE__ */ _jsx5(RichText4, {
                            __fromCanvasComponent: true,
                            children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                              children: /* @__PURE__ */ _jsx5(motion4.p, {
                                style: {
                                  '--font-selector': 'R0Y7SW50ZXItNjAw',
                                  '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                  '--framer-font-size': '14px',
                                  '--framer-font-weight': '600',
                                  '--framer-line-height': '1.5em',
                                  '--framer-text-alignment': 'left',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                },
                                children: 'month',
                              },),
                            },),
                            className: 'framer-1eek154',
                            fonts: ['GF;Inter-600',],
                            layoutDependency,
                            layoutId: 'ZzV_uhSYm',
                            style: {
                              '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                              '--framer-link-text-color': 'rgb(0, 153, 255)',
                              '--framer-link-text-decoration': 'underline',
                              '--framer-paragraph-spacing': '0px',
                            },
                            verticalAlignment: 'top',
                            withExternalLayout: true,
                          },),
                        ],
                      },),
                      /* @__PURE__ */ _jsx5(RichText4, {
                        __fromCanvasComponent: true,
                        children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                          children: /* @__PURE__ */ _jsx5(motion4.p, {
                            style: {
                              '--font-selector': 'R0Y7SW50ZXItNjAw',
                              '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                              '--framer-font-size': '14px',
                              '--framer-font-weight': '600',
                              '--framer-line-height': '1.5em',
                              '--framer-text-alignment': 'left',
                              '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                            },
                            children: '+Resource Usage',
                          },),
                        },),
                        className: 'framer-1yjee80',
                        fonts: ['GF;Inter-600',],
                        layoutDependency,
                        layoutId: 'MRHYRsVFX',
                        style: {
                          '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                          '--framer-link-text-color': 'rgb(0, 153, 255)',
                          '--framer-link-text-decoration': 'underline',
                          '--framer-paragraph-spacing': '0px',
                        },
                        verticalAlignment: 'top',
                        withExternalLayout: true,
                      },),
                      /* @__PURE__ */ _jsxs4(motion4.div, {
                        className: 'framer-r4wr8c',
                        layoutDependency,
                        layoutId: 'nmSrSQkJF',
                        children: [
                          /* @__PURE__ */ _jsxs4(motion4.div, {
                            className: 'framer-bviehl',
                            'data-framer-name': 'Row',
                            layoutDependency,
                            layoutId: 'qvkJPzjDr',
                            children: [
                              /* @__PURE__ */ _jsx5(SVG3, {
                                className: 'framer-gdinzi',
                                'data-framer-name': 'Check',
                                layout: 'position',
                                layoutDependency,
                                layoutId: 'czg_9dWKh',
                                opacity: 1,
                                radius: 0,
                                style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                svg:
                                  '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                svgContentId: 12326142209,
                                withExternalLayout: true,
                              },),
                              /* @__PURE__ */ _jsx5(RichText4, {
                                __fromCanvasComponent: true,
                                children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                  children: /* @__PURE__ */ _jsx5(motion4.p, {
                                    style: {
                                      '--font-selector': 'R0Y7SW50ZXItNjAw',
                                      '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                      '--framer-font-size': '12px',
                                      '--framer-font-weight': '600',
                                      '--framer-line-height': '1.5em',
                                      '--framer-text-alignment': 'left',
                                      '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                    },
                                    children: '2 projects',
                                  },),
                                },),
                                className: 'framer-v4tks8',
                                fonts: ['GF;Inter-600',],
                                layoutDependency,
                                layoutId: 'Ow5EMS_ky',
                                style: {
                                  '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                  '--framer-link-text-color': 'rgb(0, 153, 255)',
                                  '--framer-link-text-decoration': 'underline',
                                  '--framer-paragraph-spacing': '0px',
                                },
                                verticalAlignment: 'top',
                                withExternalLayout: true,
                              },),
                            ],
                          },),
                          /* @__PURE__ */ _jsxs4(motion4.div, {
                            className: 'framer-2wdsn5',
                            'data-framer-name': 'Row',
                            layoutDependency,
                            layoutId: 'LNSgJRjMd',
                            children: [
                              /* @__PURE__ */ _jsx5(SVG3, {
                                className: 'framer-17vg6xx',
                                'data-framer-name': 'Check',
                                layout: 'position',
                                layoutDependency,
                                layoutId: 'OR9ypV7Yc',
                                opacity: 1,
                                radius: 0,
                                style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                svg:
                                  '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                svgContentId: 12326142209,
                                withExternalLayout: true,
                              },),
                              /* @__PURE__ */ _jsx5(RichText4, {
                                __fromCanvasComponent: true,
                                children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                  children: /* @__PURE__ */ _jsx5(motion4.p, {
                                    style: {
                                      '--font-selector': 'R0Y7SW50ZXItNjAw',
                                      '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                      '--framer-font-size': '12px',
                                      '--framer-font-weight': '600',
                                      '--framer-line-height': '1.5em',
                                      '--framer-text-alignment': 'left',
                                      '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                    },
                                    children: 'Analytics',
                                  },),
                                },),
                                className: 'framer-1flrnv1',
                                fonts: ['GF;Inter-600',],
                                layoutDependency,
                                layoutId: 'dRHJaCfIy',
                                style: {
                                  '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                  '--framer-link-text-color': 'rgb(0, 153, 255)',
                                  '--framer-link-text-decoration': 'underline',
                                  '--framer-paragraph-spacing': '0px',
                                },
                                verticalAlignment: 'top',
                                withExternalLayout: true,
                              },),
                            ],
                          },),
                          /* @__PURE__ */ _jsxs4(motion4.div, {
                            className: 'framer-l7p0ro',
                            'data-framer-name': 'Row',
                            layoutDependency,
                            layoutId: 'UuxnFFeUd',
                            children: [
                              /* @__PURE__ */ _jsx5(SVG3, {
                                className: 'framer-chdw6h',
                                'data-framer-name': 'Check',
                                layout: 'position',
                                layoutDependency,
                                layoutId: 'PZ1GsfCpa',
                                opacity: 1,
                                radius: 0,
                                style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                svg:
                                  '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                svgContentId: 12326142209,
                                withExternalLayout: true,
                              },),
                              /* @__PURE__ */ _jsx5(RichText4, {
                                __fromCanvasComponent: true,
                                children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                  children: /* @__PURE__ */ _jsx5(motion4.p, {
                                    style: {
                                      '--font-selector': 'R0Y7SW50ZXItNjAw',
                                      '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                      '--framer-font-size': '12px',
                                      '--framer-font-weight': '600',
                                      '--framer-line-height': '1.5em',
                                      '--framer-text-alignment': 'left',
                                      '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                    },
                                    children: 'Email Support',
                                  },),
                                },),
                                className: 'framer-oz1xw7',
                                fonts: ['GF;Inter-600',],
                                layoutDependency,
                                layoutId: 'I9e8_bYdc',
                                style: {
                                  '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                  '--framer-link-text-color': 'rgb(0, 153, 255)',
                                  '--framer-link-text-decoration': 'underline',
                                  '--framer-paragraph-spacing': '0px',
                                },
                                verticalAlignment: 'top',
                                withExternalLayout: true,
                              },),
                            ],
                          },),
                          /* @__PURE__ */ _jsxs4(motion4.div, {
                            className: 'framer-bznku0',
                            'data-framer-name': 'Row',
                            layoutDependency,
                            layoutId: 'whJfLt8eD',
                            children: [
                              /* @__PURE__ */ _jsx5(SVG3, {
                                className: 'framer-1lbh0s6',
                                'data-framer-name': 'Check',
                                layout: 'position',
                                layoutDependency,
                                layoutId: 'F3QHCbmDa',
                                opacity: 1,
                                radius: 0,
                                style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                svg:
                                  '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                svgContentId: 12326142209,
                                withExternalLayout: true,
                              },),
                              /* @__PURE__ */ _jsx5(RichText4, {
                                __fromCanvasComponent: true,
                                children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                  children: /* @__PURE__ */ _jsx5(motion4.p, {
                                    style: {
                                      '--font-selector': 'R0Y7SW50ZXItNjAw',
                                      '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                      '--framer-font-size': '12px',
                                      '--framer-font-weight': '600',
                                      '--framer-line-height': '1.5em',
                                      '--framer-text-alignment': 'left',
                                      '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                    },
                                    children: 'Share Features',
                                  },),
                                },),
                                className: 'framer-wqoxjf',
                                fonts: ['GF;Inter-600',],
                                layoutDependency,
                                layoutId: 'zJuy6CXly',
                                style: {
                                  '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                  '--framer-link-text-color': 'rgb(0, 153, 255)',
                                  '--framer-link-text-decoration': 'underline',
                                  '--framer-paragraph-spacing': '0px',
                                },
                                verticalAlignment: 'top',
                                withExternalLayout: true,
                              },),
                            ],
                          },),
                        ],
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1nh7kfl',
                        'data-framer-name': 'Button',
                        layoutDependency,
                        layoutId: 'hXrlvYH09',
                        style: {
                          backgroundColor: 'rgb(255, 79, 0)',
                          borderBottomLeftRadius: 4,
                          borderBottomRightRadius: 4,
                          borderTopLeftRadius: 4,
                          borderTopRightRadius: 4,
                          boxShadow:
                            '0px 0.7065919983928324px 0.7065919983928324px -0.625px rgba(0, 0, 0, 0.14764), 0px 1.8065619053231785px 1.8065619053231785px -1.25px rgba(0, 0, 0, 0.14398), 0px 3.6217592146567767px 3.6217592146567767px -1.875px rgba(0, 0, 0, 0.13793), 0px 6.8655999097303715px 6.8655999097303715px -2.5px rgba(0, 0, 0, 0.12711), 0px 13.646761411524492px 13.646761411524492px -3.125px rgba(0, 0, 0, 0.10451), 0px 30px 30px -3.75px rgba(0, 0, 0, 0.05)',
                        },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--font-selector': 'R0Y7SW50ZXItNjAw',
                                '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                '--framer-font-size': '14px',
                                '--framer-font-weight': '600',
                                '--framer-letter-spacing': '0px',
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                              },
                              children: 'Get Started',
                            },),
                          },),
                          className: 'framer-1911m7a',
                          fonts: ['GF;Inter-600',],
                          layoutDependency,
                          layoutId: 'Ixkr1tcSH',
                          style: {
                            '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                            '--framer-link-text-color': 'rgb(0, 153, 255)',
                            '--framer-link-text-decoration': 'underline',
                            '--framer-paragraph-spacing': '0px',
                          },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                    ],
                  },),
                  /* @__PURE__ */ _jsxs4(motion4.div, {
                    className: 'framer-lal9la',
                    'data-border': true,
                    'data-framer-name': 'Card',
                    layoutDependency,
                    layoutId: 'l8ScxA9FZ',
                    style: {
                      '--border-bottom-width': '1px',
                      '--border-color': 'rgb(77, 77, 77)',
                      '--border-left-width': '1px',
                      '--border-right-width': '1px',
                      '--border-style': 'solid',
                      '--border-top-width': '1px',
                      backgroundColor: 'rgb(8, 8, 8)',
                      borderBottomLeftRadius: 6,
                      borderBottomRightRadius: 6,
                      borderTopLeftRadius: 6,
                      borderTopRightRadius: 6,
                      boxShadow:
                        '0px 0.7961918735236395px 2.3885756205709185px -0.625px rgba(0, 0, 0, 0.05), 0px 2.414506143104518px 7.2435184293135535px -1.25px rgba(0, 0, 0, 0.05), 0px 6.382653521484461px 19.147960564453385px -1.875px rgba(0, 0, 0, 0.05), 0px 20px 60px -2.5px rgba(0, 0, 0, 0.05)',
                    },
                    children: [
                      /* @__PURE__ */ _jsx5(RichText4, {
                        __fromCanvasComponent: true,
                        children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                          children: /* @__PURE__ */ _jsx5(motion4.p, {
                            style: {
                              '--font-selector': 'R0Y7SW50ZXItNjAw',
                              '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                              '--framer-font-size': '24px',
                              '--framer-font-weight': '600',
                              '--framer-line-height': '1.5em',
                              '--framer-text-alignment': 'left',
                              '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                            },
                            children: 'Team',
                          },),
                        },),
                        className: 'framer-17ciarf',
                        fonts: ['GF;Inter-600',],
                        layoutDependency,
                        layoutId: 'IasjBze76',
                        style: {
                          '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                          '--framer-link-text-color': 'rgb(0, 153, 255)',
                          '--framer-link-text-decoration': 'underline',
                          '--framer-paragraph-spacing': '0px',
                        },
                        verticalAlignment: 'top',
                        withExternalLayout: true,
                      },),
                      /* @__PURE__ */ _jsxs4(motion4.div, {
                        className: 'framer-175pznh',
                        layoutDependency,
                        layoutId: 'DbPaiJdZF',
                        children: [
                          /* @__PURE__ */ _jsx5(RichText4, {
                            __fromCanvasComponent: true,
                            children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                              children: /* @__PURE__ */ _jsx5(motion4.p, {
                                style: {
                                  '--font-selector': 'R0Y7SW50ZXItNzAw',
                                  '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                  '--framer-font-size': '40px',
                                  '--framer-font-weight': '700',
                                  '--framer-letter-spacing': '-3px',
                                  '--framer-line-height': '1em',
                                  '--framer-text-alignment': 'left',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                                },
                                children: '$200',
                              },),
                            },),
                            className: 'framer-18p5km5',
                            fonts: ['GF;Inter-700',],
                            layoutDependency,
                            layoutId: 'PiyqqdjWu',
                            style: {
                              '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                              '--framer-link-text-color': 'rgb(0, 153, 255)',
                              '--framer-link-text-decoration': 'underline',
                              '--framer-paragraph-spacing': '0px',
                            },
                            verticalAlignment: 'top',
                            withExternalLayout: true,
                          },),
                          /* @__PURE__ */ _jsx5(RichText4, {
                            __fromCanvasComponent: true,
                            children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                              children: /* @__PURE__ */ _jsx5(motion4.p, {
                                style: {
                                  '--font-selector': 'R0Y7SW50ZXItNjAw',
                                  '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                  '--framer-font-size': '14px',
                                  '--framer-font-weight': '600',
                                  '--framer-line-height': '1.5em',
                                  '--framer-text-alignment': 'left',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                },
                                children: 'month',
                              },),
                            },),
                            className: 'framer-1v49zt5',
                            fonts: ['GF;Inter-600',],
                            layoutDependency,
                            layoutId: 'cVUxXpv5s',
                            style: {
                              '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                              '--framer-link-text-color': 'rgb(0, 153, 255)',
                              '--framer-link-text-decoration': 'underline',
                              '--framer-paragraph-spacing': '0px',
                            },
                            verticalAlignment: 'top',
                            withExternalLayout: true,
                          },),
                        ],
                      },),
                      /* @__PURE__ */ _jsx5(RichText4, {
                        __fromCanvasComponent: true,
                        children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                          children: /* @__PURE__ */ _jsx5(motion4.p, {
                            style: {
                              '--font-selector': 'R0Y7SW50ZXItNjAw',
                              '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                              '--framer-font-size': '12px',
                              '--framer-font-weight': '600',
                              '--framer-line-height': '1.5em',
                              '--framer-text-alignment': 'left',
                              '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                            },
                            children: '+Resource Usage',
                          },),
                        },),
                        className: 'framer-2nq4yf',
                        fonts: ['GF;Inter-600',],
                        layoutDependency,
                        layoutId: 'sdOuKVVVh',
                        style: {
                          '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                          '--framer-link-text-color': 'rgb(0, 153, 255)',
                          '--framer-link-text-decoration': 'underline',
                          '--framer-paragraph-spacing': '0px',
                        },
                        verticalAlignment: 'top',
                        withExternalLayout: true,
                      },),
                      /* @__PURE__ */ _jsxs4(motion4.div, {
                        className: 'framer-pipxka',
                        layoutDependency,
                        layoutId: 'KuCdaxVzb',
                        children: [
                          /* @__PURE__ */ _jsxs4(motion4.div, {
                            className: 'framer-3rb4qb',
                            'data-framer-name': 'Row',
                            layoutDependency,
                            layoutId: 's0dj5cn35',
                            children: [
                              /* @__PURE__ */ _jsx5(SVG3, {
                                className: 'framer-1rypvf',
                                'data-framer-name': 'Check',
                                layout: 'position',
                                layoutDependency,
                                layoutId: 'Dib5wfCyr',
                                opacity: 1,
                                radius: 0,
                                style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                svg:
                                  '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                svgContentId: 12129822493,
                                withExternalLayout: true,
                              },),
                              /* @__PURE__ */ _jsx5(RichText4, {
                                __fromCanvasComponent: true,
                                children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                  children: /* @__PURE__ */ _jsx5(motion4.p, {
                                    style: {
                                      '--font-selector': 'R0Y7SW50ZXItNjAw',
                                      '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                      '--framer-font-size': '12px',
                                      '--framer-font-weight': '600',
                                      '--framer-line-height': '1.5em',
                                      '--framer-text-alignment': 'left',
                                      '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                    },
                                    children: 'AWS + G Cloud + Azure',
                                  },),
                                },),
                                className: 'framer-1wg266f',
                                fonts: ['GF;Inter-600',],
                                layoutDependency,
                                layoutId: 'mHS7NXYt_',
                                style: {
                                  '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                  '--framer-link-text-color': 'rgb(0, 153, 255)',
                                  '--framer-link-text-decoration': 'underline',
                                  '--framer-paragraph-spacing': '0px',
                                },
                                verticalAlignment: 'top',
                                withExternalLayout: true,
                              },),
                            ],
                          },),
                          /* @__PURE__ */ _jsxs4(motion4.div, {
                            className: 'framer-1odf1b2',
                            'data-framer-name': 'Row',
                            layoutDependency,
                            layoutId: 'wNsDkJ9JA',
                            children: [
                              /* @__PURE__ */ _jsx5(SVG3, {
                                className: 'framer-1o4uz2h',
                                'data-framer-name': 'Check',
                                layout: 'position',
                                layoutDependency,
                                layoutId: 'i4Ar_Y58y',
                                opacity: 1,
                                radius: 0,
                                style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                svg:
                                  '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                svgContentId: 12129822493,
                                withExternalLayout: true,
                              },),
                              /* @__PURE__ */ _jsx5(RichText4, {
                                __fromCanvasComponent: true,
                                children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                  children: /* @__PURE__ */ _jsx5(motion4.p, {
                                    style: {
                                      '--font-selector': 'R0Y7SW50ZXItNjAw',
                                      '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                      '--framer-font-size': '12px',
                                      '--framer-font-weight': '600',
                                      '--framer-line-height': '1.5em',
                                      '--framer-text-alignment': 'left',
                                      '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                    },
                                    children: 'Analytics',
                                  },),
                                },),
                                className: 'framer-1wb23t5',
                                fonts: ['GF;Inter-600',],
                                layoutDependency,
                                layoutId: 'bHpYA2IXT',
                                style: {
                                  '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                  '--framer-link-text-color': 'rgb(0, 153, 255)',
                                  '--framer-link-text-decoration': 'underline',
                                  '--framer-paragraph-spacing': '0px',
                                },
                                verticalAlignment: 'top',
                                withExternalLayout: true,
                              },),
                            ],
                          },),
                          /* @__PURE__ */ _jsxs4(motion4.div, {
                            className: 'framer-111ge6c',
                            'data-framer-name': 'Row',
                            layoutDependency,
                            layoutId: 'wBsY5UsLb',
                            children: [
                              /* @__PURE__ */ _jsx5(SVG3, {
                                className: 'framer-1d5md58',
                                'data-framer-name': 'Check',
                                layout: 'position',
                                layoutDependency,
                                layoutId: 'MRvbljvj9',
                                opacity: 1,
                                radius: 0,
                                style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                svg:
                                  '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                svgContentId: 12129822493,
                                withExternalLayout: true,
                              },),
                              /* @__PURE__ */ _jsx5(RichText4, {
                                __fromCanvasComponent: true,
                                children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                  children: /* @__PURE__ */ _jsx5(motion4.p, {
                                    style: {
                                      '--font-selector': 'R0Y7SW50ZXItNjAw',
                                      '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                      '--framer-font-size': '12px',
                                      '--framer-font-weight': '600',
                                      '--framer-line-height': '1.5em',
                                      '--framer-text-alignment': 'left',
                                      '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                    },
                                    children: 'Advance Support',
                                  },),
                                },),
                                className: 'framer-18gt7p0',
                                fonts: ['GF;Inter-600',],
                                layoutDependency,
                                layoutId: 'Gx3gc1VTC',
                                style: {
                                  '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                  '--framer-link-text-color': 'rgb(0, 153, 255)',
                                  '--framer-link-text-decoration': 'underline',
                                  '--framer-paragraph-spacing': '0px',
                                },
                                verticalAlignment: 'top',
                                withExternalLayout: true,
                              },),
                            ],
                          },),
                          /* @__PURE__ */ _jsxs4(motion4.div, {
                            className: 'framer-vapxzp',
                            'data-framer-name': 'Row',
                            layoutDependency,
                            layoutId: 'OQkXA9NaE',
                            children: [
                              /* @__PURE__ */ _jsx5(SVG3, {
                                className: 'framer-dtblfb',
                                'data-framer-name': 'Check',
                                layout: 'position',
                                layoutDependency,
                                layoutId: 'KVHzYoLKk',
                                opacity: 1,
                                radius: 0,
                                style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                svg:
                                  '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                svgContentId: 12129822493,
                                withExternalLayout: true,
                              },),
                              /* @__PURE__ */ _jsx5(RichText4, {
                                __fromCanvasComponent: true,
                                children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                  children: /* @__PURE__ */ _jsx5(motion4.p, {
                                    style: {
                                      '--font-selector': 'R0Y7SW50ZXItNjAw',
                                      '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                      '--framer-font-size': '12px',
                                      '--framer-font-weight': '600',
                                      '--framer-line-height': '1.5em',
                                      '--framer-text-alignment': 'left',
                                      '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                    },
                                    children: 'Share Features',
                                  },),
                                },),
                                className: 'framer-i7k76n',
                                fonts: ['GF;Inter-600',],
                                layoutDependency,
                                layoutId: 'lgBQesrit',
                                style: {
                                  '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                  '--framer-link-text-color': 'rgb(0, 153, 255)',
                                  '--framer-link-text-decoration': 'underline',
                                  '--framer-paragraph-spacing': '0px',
                                },
                                verticalAlignment: 'top',
                                withExternalLayout: true,
                              },),
                            ],
                          },),
                        ],
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1dlm72w',
                        'data-framer-name': 'Button',
                        layoutDependency,
                        layoutId: 'MOakODWrE',
                        style: {
                          backgroundColor: 'rgb(255, 79, 0)',
                          borderBottomLeftRadius: 4,
                          borderBottomRightRadius: 4,
                          borderTopLeftRadius: 4,
                          borderTopRightRadius: 4,
                          boxShadow:
                            '0px 0.7065919983928324px 0.7065919983928324px -0.625px rgba(0, 0, 0, 0.14764), 0px 1.8065619053231785px 1.8065619053231785px -1.25px rgba(0, 0, 0, 0.14398), 0px 3.6217592146567767px 3.6217592146567767px -1.875px rgba(0, 0, 0, 0.13793), 0px 6.8655999097303715px 6.8655999097303715px -2.5px rgba(0, 0, 0, 0.12711), 0px 13.646761411524492px 13.646761411524492px -3.125px rgba(0, 0, 0, 0.10451), 0px 30px 30px -3.75px rgba(0, 0, 0, 0.05)',
                        },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--font-selector': 'R0Y7SW50ZXItNjAw',
                                '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                '--framer-font-size': '14px',
                                '--framer-font-weight': '600',
                                '--framer-letter-spacing': '0px',
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                              },
                              children: 'Get Started',
                            },),
                          },),
                          className: 'framer-fmfcsj',
                          fonts: ['GF;Inter-600',],
                          layoutDependency,
                          layoutId: 'IEE2yqc15',
                          style: {
                            '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                            '--framer-link-text-color': 'rgb(0, 153, 255)',
                            '--framer-link-text-decoration': 'underline',
                            '--framer-paragraph-spacing': '0px',
                          },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                    ],
                  },),
                  /* @__PURE__ */ _jsxs4(motion4.div, {
                    className: 'framer-9qmgde',
                    'data-border': true,
                    'data-framer-name': 'Card',
                    layoutDependency,
                    layoutId: 'SIV58AAbG',
                    style: {
                      '--border-bottom-width': '1px',
                      '--border-color': 'rgb(77, 77, 77)',
                      '--border-left-width': '1px',
                      '--border-right-width': '1px',
                      '--border-style': 'solid',
                      '--border-top-width': '1px',
                      backgroundColor: 'rgb(8, 8, 8)',
                      borderBottomLeftRadius: 6,
                      borderBottomRightRadius: 6,
                      borderTopLeftRadius: 6,
                      borderTopRightRadius: 6,
                      boxShadow:
                        '0px 0.7961918735236395px 2.3885756205709185px -0.625px rgba(0, 0, 0, 0.05), 0px 2.414506143104518px 7.2435184293135535px -1.25px rgba(0, 0, 0, 0.05), 0px 6.382653521484461px 19.147960564453385px -1.875px rgba(0, 0, 0, 0.05), 0px 20px 60px -2.5px rgba(0, 0, 0, 0.05)',
                    },
                    children: [
                      /* @__PURE__ */ _jsx5(RichText4, {
                        __fromCanvasComponent: true,
                        children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                          children: /* @__PURE__ */ _jsx5(motion4.p, {
                            style: {
                              '--font-selector': 'R0Y7SW50ZXItNjAw',
                              '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                              '--framer-font-size': '24px',
                              '--framer-font-weight': '600',
                              '--framer-line-height': '1.5em',
                              '--framer-text-alignment': 'left',
                              '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                            },
                            children: 'Enterprise',
                          },),
                        },),
                        className: 'framer-vcu553',
                        fonts: ['GF;Inter-600',],
                        layoutDependency,
                        layoutId: 'LOJzbULba',
                        style: {
                          '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                          '--framer-link-text-color': 'rgb(0, 153, 255)',
                          '--framer-link-text-decoration': 'underline',
                          '--framer-paragraph-spacing': '0px',
                        },
                        verticalAlignment: 'top',
                        withExternalLayout: true,
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-2yntvv',
                        layoutDependency,
                        layoutId: 'eRFJZw67y',
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--font-selector': 'R0Y7SW50ZXItNzAw',
                                '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                '--framer-font-size': '40px',
                                '--framer-font-weight': '700',
                                '--framer-letter-spacing': '-3px',
                                '--framer-line-height': '1em',
                                '--framer-text-alignment': 'left',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                              },
                              children: 'Custom',
                            },),
                          },),
                          className: 'framer-1ais9no',
                          fonts: ['GF;Inter-700',],
                          layoutDependency,
                          layoutId: 'IwaLes7EW',
                          style: {
                            '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                            '--framer-link-text-color': 'rgb(0, 153, 255)',
                            '--framer-link-text-decoration': 'underline',
                            '--framer-paragraph-spacing': '0px',
                          },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(RichText4, {
                        __fromCanvasComponent: true,
                        children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                          children: /* @__PURE__ */ _jsx5(motion4.p, {
                            style: {
                              '--font-selector': 'R0Y7SW50ZXItNjAw',
                              '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                              '--framer-font-size': '14px',
                              '--framer-font-weight': '600',
                              '--framer-line-height': '1.5em',
                              '--framer-text-alignment': 'left',
                              '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                            },
                            children: ' ',
                          },),
                        },),
                        className: 'framer-12v1cfv',
                        fonts: ['GF;Inter-600',],
                        layoutDependency,
                        layoutId: 'tSYOh55Ta',
                        style: {
                          '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                          '--framer-link-text-color': 'rgb(0, 153, 255)',
                          '--framer-link-text-decoration': 'underline',
                          '--framer-paragraph-spacing': '0px',
                        },
                        verticalAlignment: 'top',
                        withExternalLayout: true,
                      },),
                      /* @__PURE__ */ _jsxs4(motion4.div, {
                        className: 'framer-1nwrcuf',
                        layoutDependency,
                        layoutId: 'FFXqkzoiI',
                        children: [
                          /* @__PURE__ */ _jsxs4(motion4.div, {
                            className: 'framer-czip0k',
                            'data-framer-name': 'Row',
                            layoutDependency,
                            layoutId: 'koMwHB20g',
                            children: [
                              /* @__PURE__ */ _jsx5(SVG3, {
                                className: 'framer-1p3y9rt',
                                'data-framer-name': 'Check',
                                layout: 'position',
                                layoutDependency,
                                layoutId: 'f_uOg64mE',
                                opacity: 1,
                                radius: 0,
                                style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                svg:
                                  '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                svgContentId: 12129822493,
                                withExternalLayout: true,
                              },),
                              /* @__PURE__ */ _jsx5(RichText4, {
                                __fromCanvasComponent: true,
                                children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                  children: /* @__PURE__ */ _jsx5(motion4.p, {
                                    style: {
                                      '--font-selector': 'R0Y7SW50ZXItNjAw',
                                      '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                      '--framer-font-size': '12px',
                                      '--framer-font-weight': '600',
                                      '--framer-line-height': '1.5em',
                                      '--framer-text-alignment': 'left',
                                      '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                    },
                                    children: 'Unlimited Projects',
                                  },),
                                },),
                                className: 'framer-hedmwj',
                                fonts: ['GF;Inter-600',],
                                layoutDependency,
                                layoutId: 'bXFXychQl',
                                style: {
                                  '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                  '--framer-link-text-color': 'rgb(0, 153, 255)',
                                  '--framer-link-text-decoration': 'underline',
                                  '--framer-paragraph-spacing': '0px',
                                },
                                verticalAlignment: 'top',
                                withExternalLayout: true,
                              },),
                            ],
                          },),
                          /* @__PURE__ */ _jsxs4(motion4.div, {
                            className: 'framer-1jixfcb',
                            'data-framer-name': 'Row',
                            layoutDependency,
                            layoutId: 'zQWc7MdcK',
                            children: [
                              /* @__PURE__ */ _jsx5(SVG3, {
                                className: 'framer-1wsyad2',
                                'data-framer-name': 'Check',
                                layout: 'position',
                                layoutDependency,
                                layoutId: 'okz__mriM',
                                opacity: 1,
                                radius: 0,
                                style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                svg:
                                  '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                svgContentId: 12129822493,
                                withExternalLayout: true,
                              },),
                              /* @__PURE__ */ _jsx5(RichText4, {
                                __fromCanvasComponent: true,
                                children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                  children: /* @__PURE__ */ _jsx5(motion4.p, {
                                    style: {
                                      '--font-selector': 'R0Y7SW50ZXItNjAw',
                                      '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                      '--framer-font-size': '12px',
                                      '--framer-font-weight': '600',
                                      '--framer-line-height': '1.5em',
                                      '--framer-text-alignment': 'left',
                                      '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                    },
                                    children: 'Priority Support',
                                  },),
                                },),
                                className: 'framer-1856kn0',
                                fonts: ['GF;Inter-600',],
                                layoutDependency,
                                layoutId: 'WUmYbfC_6',
                                style: {
                                  '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                  '--framer-link-text-color': 'rgb(0, 153, 255)',
                                  '--framer-link-text-decoration': 'underline',
                                  '--framer-paragraph-spacing': '0px',
                                },
                                verticalAlignment: 'top',
                                withExternalLayout: true,
                              },),
                            ],
                          },),
                          /* @__PURE__ */ _jsxs4(motion4.div, {
                            className: 'framer-gx8cy0',
                            'data-framer-name': 'Row',
                            layoutDependency,
                            layoutId: 'e_Md8_li6',
                            children: [
                              /* @__PURE__ */ _jsx5(SVG3, {
                                className: 'framer-1himl70',
                                'data-framer-name': 'Check',
                                layout: 'position',
                                layoutDependency,
                                layoutId: 'oyx4C0b0b',
                                opacity: 1,
                                radius: 0,
                                style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                svg:
                                  '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                svgContentId: 12129822493,
                                withExternalLayout: true,
                              },),
                              /* @__PURE__ */ _jsx5(RichText4, {
                                __fromCanvasComponent: true,
                                children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                  children: /* @__PURE__ */ _jsx5(motion4.p, {
                                    style: {
                                      '--font-selector': 'R0Y7SW50ZXItNjAw',
                                      '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                      '--framer-font-size': '12px',
                                      '--framer-font-weight': '600',
                                      '--framer-line-height': '1.5em',
                                      '--framer-text-alignment': 'left',
                                      '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                    },
                                    children: '99.99% SLA',
                                  },),
                                },),
                                className: 'framer-1oz1rxo',
                                fonts: ['GF;Inter-600',],
                                layoutDependency,
                                layoutId: 'bLbegRtCz',
                                style: {
                                  '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                  '--framer-link-text-color': 'rgb(0, 153, 255)',
                                  '--framer-link-text-decoration': 'underline',
                                  '--framer-paragraph-spacing': '0px',
                                },
                                verticalAlignment: 'top',
                                withExternalLayout: true,
                              },),
                            ],
                          },),
                          /* @__PURE__ */ _jsxs4(motion4.div, {
                            className: 'framer-1lr6op5',
                            'data-framer-name': 'Row',
                            layoutDependency,
                            layoutId: 'f8j2IL1D1',
                            children: [
                              /* @__PURE__ */ _jsx5(SVG3, {
                                className: 'framer-q5ktmh',
                                'data-framer-name': 'Check',
                                layout: 'position',
                                layoutDependency,
                                layoutId: 'GASJEHxqB',
                                opacity: 1,
                                radius: 0,
                                style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                svg:
                                  '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                svgContentId: 12129822493,
                                withExternalLayout: true,
                              },),
                              /* @__PURE__ */ _jsx5(RichText4, {
                                __fromCanvasComponent: true,
                                children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                  children: /* @__PURE__ */ _jsx5(motion4.p, {
                                    style: {
                                      '--font-selector': 'R0Y7SW50ZXItNjAw',
                                      '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                      '--framer-font-size': '12px',
                                      '--framer-font-weight': '600',
                                      '--framer-line-height': '1.5em',
                                      '--framer-text-alignment': 'left',
                                      '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                    },
                                    children: 'OIDC SSO provider',
                                  },),
                                },),
                                className: 'framer-lad5xu',
                                fonts: ['GF;Inter-600',],
                                layoutDependency,
                                layoutId: 'W8uAcux2J',
                                style: {
                                  '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                  '--framer-link-text-color': 'rgb(0, 153, 255)',
                                  '--framer-link-text-decoration': 'underline',
                                  '--framer-paragraph-spacing': '0px',
                                },
                                verticalAlignment: 'top',
                                withExternalLayout: true,
                              },),
                            ],
                          },),
                        ],
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-zgdo2p',
                        'data-framer-name': 'Button',
                        layoutDependency,
                        layoutId: 'l9kjjdmuX',
                        style: {
                          backgroundColor: 'rgb(255, 79, 0)',
                          borderBottomLeftRadius: 4,
                          borderBottomRightRadius: 4,
                          borderTopLeftRadius: 4,
                          borderTopRightRadius: 4,
                          boxShadow:
                            '0px 0.7065919983928324px 0.7065919983928324px -0.625px rgba(0, 0, 0, 0.14764), 0px 1.8065619053231785px 1.8065619053231785px -1.25px rgba(0, 0, 0, 0.14398), 0px 3.6217592146567767px 3.6217592146567767px -1.875px rgba(0, 0, 0, 0.13793), 0px 6.8655999097303715px 6.8655999097303715px -2.5px rgba(0, 0, 0, 0.12711), 0px 13.646761411524492px 13.646761411524492px -3.125px rgba(0, 0, 0, 0.10451), 0px 30px 30px -3.75px rgba(0, 0, 0, 0.05)',
                        },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--font-selector': 'R0Y7SW50ZXItNjAw',
                                '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                '--framer-font-size': '14px',
                                '--framer-font-weight': '600',
                                '--framer-letter-spacing': '0px',
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                              },
                              children: 'Contact Us',
                            },),
                          },),
                          className: 'framer-i71x2i',
                          fonts: ['GF;Inter-600',],
                          layoutDependency,
                          layoutId: 'WOndxV39W',
                          style: {
                            '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                            '--framer-link-text-color': 'rgb(0, 153, 255)',
                            '--framer-link-text-decoration': 'underline',
                            '--framer-paragraph-spacing': '0px',
                          },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                    ],
                  },),
                ],
              },),
            },),
            isDisplayed() && /* @__PURE__ */ _jsx5(motion4.div, {
              className: 'framer-1s0vgn0',
              layoutDependency,
              layoutId: 'gPPAmfCmJ',
              children: /* @__PURE__ */ _jsxs4(motion4.div, {
                className: 'framer-1sksdhj',
                layoutDependency,
                layoutId: 'ukHjDrPZZ',
                children: [
                  /* @__PURE__ */ _jsxs4(motion4.div, {
                    className: 'framer-1hest51',
                    layoutDependency,
                    layoutId: 'sRHlsx7Qr',
                    children: [
                      /* @__PURE__ */ _jsx5(RichText4, {
                        __fromCanvasComponent: true,
                        children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                          children: /* @__PURE__ */ _jsx5(motion4.h3, {
                            style: {
                              '--font-selector': 'SW50ZXItQm9sZA==',
                              '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                              '--framer-font-size': '24px',
                              '--framer-font-weight': '700',
                              '--framer-letter-spacing': '-1.9px',
                              '--framer-text-alignment': 'left',
                              '--framer-text-color': 'var(--extracted-a0htzi, rgb(255, 255, 255))',
                            },
                            children: 'Plan Features',
                          },),
                        },),
                        className: 'framer-1clngzw',
                        fonts: ['Inter-Bold',],
                        layoutDependency,
                        layoutId: 'aqfd0BLGP',
                        style: {
                          '--extracted-a0htzi': 'rgb(255, 255, 255)',
                          '--framer-link-text-color': 'rgb(0, 153, 255)',
                          '--framer-link-text-decoration': 'underline',
                          '--framer-paragraph-spacing': '0px',
                        },
                        verticalAlignment: 'top',
                        withExternalLayout: true,
                      },),
                      /* @__PURE__ */ _jsx5(RichText4, {
                        __fromCanvasComponent: true,
                        children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                          children: /* @__PURE__ */ _jsx5(motion4.p, {
                            style: {
                              '--framer-text-alignment': 'left',
                              '--framer-text-color': 'var(--extracted-r6o4lv, rgb(138, 138, 138))',
                            },
                            children: 'Start for free with scalable pricing as you grow.',
                          },),
                        },),
                        className: 'framer-1jxboe2',
                        fonts: ['Inter',],
                        layoutDependency,
                        layoutId: 'p4AfaZWdU',
                        style: { '--extracted-r6o4lv': 'rgb(138, 138, 138)', },
                        verticalAlignment: 'top',
                        withExternalLayout: true,
                      },),
                    ],
                  },),
                  /* @__PURE__ */ _jsxs4(motion4.div, {
                    className: 'framer-k3iqma',
                    layoutDependency,
                    layoutId: 'yn2xElXFq',
                    children: [
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-99xr8r',
                        layoutDependency,
                        layoutId: 'IeC44tWrZ',
                        style: { backgroundColor: 'rgba(186, 221, 255, 0)', },
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1qnnkps',
                        layoutDependency,
                        layoutId: 'YS6ElXHMy',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--font-selector': 'SW50ZXItQm9sZA==',
                                '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                '--framer-font-weight': '700',
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                              },
                              children: 'Community',
                            },),
                          },),
                          className: 'framer-1gayotb',
                          fonts: ['Inter-Bold',],
                          layoutDependency,
                          layoutId: 'U53l1t1Rn',
                          style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1qczvql',
                        layoutDependency,
                        layoutId: 'wc3mY83Q5',
                        style: { backgroundColor: 'rgba(255, 255, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--font-selector': 'SW50ZXItQm9sZA==',
                                '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                '--framer-font-weight': '700',
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                              },
                              children: 'Pro',
                            },),
                          },),
                          className: 'framer-tjuho',
                          fonts: ['Inter-Bold',],
                          layoutDependency,
                          layoutId: 'hv961l3E4',
                          style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-8czh2',
                        layoutDependency,
                        layoutId: 'VCYqzyZfW',
                        style: { backgroundColor: 'rgba(186, 221, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--font-selector': 'SW50ZXItQm9sZA==',
                                '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                '--framer-font-weight': '700',
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                              },
                              children: 'Team',
                            },),
                          },),
                          className: 'framer-7enftg',
                          fonts: ['Inter-Bold',],
                          layoutDependency,
                          layoutId: 'EqCI0v564',
                          style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-5ya5nz',
                        layoutDependency,
                        layoutId: 'pqxuuo4N2',
                        style: { backgroundColor: 'rgba(111, 136, 161, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--font-selector': 'SW50ZXItQm9sZA==',
                                '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                '--framer-font-weight': '700',
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                              },
                              children: 'Enterprise',
                            },),
                          },),
                          className: 'framer-zioe9h',
                          fonts: ['Inter-Bold',],
                          layoutDependency,
                          layoutId: 'ep2C2IDzc',
                          style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(RichText4, {
                        __fromCanvasComponent: true,
                        children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                          children: /* @__PURE__ */ _jsx5(motion4.p, {
                            style: {
                              '--font-selector': 'R0Y7SW50ZXItNzAw',
                              '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                              '--framer-font-weight': '700',
                              '--framer-line-height': '1.5em',
                              '--framer-text-alignment': 'left',
                              '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                            },
                            children: 'Log Retention',
                          },),
                        },),
                        className: 'framer-1miy0c7',
                        fonts: ['GF;Inter-700',],
                        layoutDependency,
                        layoutId: 'kXQW0ij4c',
                        style: {
                          '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                          '--framer-link-text-color': 'rgb(0, 153, 255)',
                          '--framer-link-text-decoration': 'underline',
                          '--framer-paragraph-spacing': '0px',
                        },
                        verticalAlignment: 'center',
                        withExternalLayout: true,
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1ig55ua',
                        layoutDependency,
                        layoutId: 'IOZKK25tv',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                              },
                              children: '24 Hours',
                            },),
                          },),
                          className: 'framer-9uhagv',
                          fonts: ['Inter',],
                          layoutDependency,
                          layoutId: 'kKbXOn_PC',
                          style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1omxy4d',
                        layoutDependency,
                        layoutId: 'LkUXybg80',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                              },
                              children: '14 Days',
                            },),
                          },),
                          className: 'framer-1jfn7c',
                          fonts: ['Inter',],
                          layoutDependency,
                          layoutId: 'q1JeNUPWZ',
                          style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1gn92qy',
                        layoutDependency,
                        layoutId: 'MnoVGElJ4',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                              },
                              children: '30 Days',
                            },),
                          },),
                          className: 'framer-1pjiik1',
                          fonts: ['Inter',],
                          layoutDependency,
                          layoutId: 'jwfMF3lrl',
                          style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-ro7tfu',
                        layoutDependency,
                        layoutId: 'IkLB_sEm1',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                              },
                              children: '6 Months',
                            },),
                          },),
                          className: 'framer-qu0m4x',
                          fonts: ['Inter',],
                          layoutDependency,
                          layoutId: 'KjBJAkXPD',
                          style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(RichText4, {
                        __fromCanvasComponent: true,
                        children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                          children: /* @__PURE__ */ _jsx5(motion4.p, {
                            style: {
                              '--font-selector': 'R0Y7SW50ZXItNzAw',
                              '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                              '--framer-font-weight': '700',
                              '--framer-line-height': '1.5em',
                              '--framer-text-alignment': 'left',
                              '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                            },
                            children: 'Build Retention',
                          },),
                        },),
                        className: 'framer-pc9xpx',
                        fonts: ['GF;Inter-700',],
                        layoutDependency,
                        layoutId: 'PTJ2b8jTE',
                        style: {
                          '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                          '--framer-link-text-color': 'rgb(0, 153, 255)',
                          '--framer-link-text-decoration': 'underline',
                          '--framer-paragraph-spacing': '0px',
                        },
                        verticalAlignment: 'center',
                        withExternalLayout: true,
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-t1t9y9',
                        layoutDependency,
                        layoutId: 'FiKoOCffs',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                              },
                              children: '24 Hours',
                            },),
                          },),
                          className: 'framer-1ypvbg5',
                          fonts: ['Inter',],
                          layoutDependency,
                          layoutId: 'hXuim02YW',
                          style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1rsdhpb',
                        layoutDependency,
                        layoutId: 'RLpx9eGhb',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                              },
                              children: '14 Days',
                            },),
                          },),
                          className: 'framer-yelzwc',
                          fonts: ['Inter',],
                          layoutDependency,
                          layoutId: 'M2OKQnyeO',
                          style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1i23qxm',
                        layoutDependency,
                        layoutId: 'HVPj3TB2H',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                              },
                              children: '30 Days',
                            },),
                          },),
                          className: 'framer-1y4hjz8',
                          fonts: ['Inter',],
                          layoutDependency,
                          layoutId: 'jq88tRtpe',
                          style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-11k1b7b',
                        layoutDependency,
                        layoutId: 'BB_sFIR8S',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                              },
                              children: '6 Months',
                            },),
                          },),
                          className: 'framer-sneo2m',
                          fonts: ['Inter',],
                          layoutDependency,
                          layoutId: 'je23i689F',
                          style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(RichText4, {
                        __fromCanvasComponent: true,
                        children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                          children: /* @__PURE__ */ _jsx5(motion4.p, {
                            style: {
                              '--font-selector': 'R0Y7SW50ZXItNzAw',
                              '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                              '--framer-font-weight': '700',
                              '--framer-line-height': '1.5em',
                              '--framer-text-alignment': 'left',
                              '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                            },
                            children: 'Support',
                          },),
                        },),
                        className: 'framer-dkecy8',
                        fonts: ['GF;Inter-700',],
                        layoutDependency,
                        layoutId: 'bzXql7ICF',
                        style: {
                          '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                          '--framer-link-text-color': 'rgb(0, 153, 255)',
                          '--framer-link-text-decoration': 'underline',
                          '--framer-paragraph-spacing': '0px',
                        },
                        verticalAlignment: 'center',
                        withExternalLayout: true,
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1y0qryh',
                        layoutDependency,
                        layoutId: 'BsMBUw9qS',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                              },
                              children: 'Community Support',
                            },),
                          },),
                          className: 'framer-trwkm1',
                          fonts: ['Inter',],
                          layoutDependency,
                          layoutId: 'hyzi70IgP',
                          style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1aboik7',
                        layoutDependency,
                        layoutId: 'SPucAqCqS',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                              },
                              children: 'Email',
                            },),
                          },),
                          className: 'framer-l4r5u9',
                          fonts: ['Inter',],
                          layoutDependency,
                          layoutId: 'dUWE5WO2I',
                          style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-a82jx1',
                        layoutDependency,
                        layoutId: 'kCxq1uqSl',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                              },
                              children: 'Priority Email',
                            },),
                          },),
                          className: 'framer-1uivg47',
                          fonts: ['Inter',],
                          layoutDependency,
                          layoutId: 'O7vLOvTrf',
                          style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1ytxh3g',
                        layoutDependency,
                        layoutId: 'yImRNiLPf',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                              },
                              children: 'Chat & Email',
                            },),
                          },),
                          className: 'framer-x597qf',
                          fonts: ['Inter',],
                          layoutDependency,
                          layoutId: 'jOtIQX65e',
                          style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                    ],
                  },),
                ],
              },),
            },),
            isDisplayed() && /* @__PURE__ */ _jsx5(motion4.div, {
              className: 'framer-18fwcn',
              layoutDependency,
              layoutId: 'U6lDf3ciu',
              children: /* @__PURE__ */ _jsxs4(motion4.div, {
                className: 'framer-12vp7i5',
                layoutDependency,
                layoutId: 'Hj70aScY5',
                children: [
                  /* @__PURE__ */ _jsxs4(motion4.div, {
                    className: 'framer-d5yvmc',
                    layoutDependency,
                    layoutId: 'kzhqJ0xqC',
                    children: [
                      /* @__PURE__ */ _jsx5(RichText4, {
                        __fromCanvasComponent: true,
                        children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                          children: /* @__PURE__ */ _jsx5(motion4.h3, {
                            style: {
                              '--font-selector': 'SW50ZXItQm9sZA==',
                              '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                              '--framer-font-size': '24px',
                              '--framer-font-weight': '700',
                              '--framer-letter-spacing': '-1.9px',
                              '--framer-text-alignment': 'left',
                              '--framer-text-color': 'var(--extracted-a0htzi, rgb(255, 255, 255))',
                            },
                            children: 'On-Demand Hardware',
                          },),
                        },),
                        className: 'framer-921me1',
                        fonts: ['Inter-Bold',],
                        layoutDependency,
                        layoutId: 'jg32Vc5tj',
                        style: {
                          '--extracted-a0htzi': 'rgb(255, 255, 255)',
                          '--framer-link-text-color': 'rgb(0, 153, 255)',
                          '--framer-link-text-decoration': 'underline',
                          '--framer-paragraph-spacing': '0px',
                        },
                        verticalAlignment: 'top',
                        withExternalLayout: true,
                      },),
                      /* @__PURE__ */ _jsx5(RichText4, {
                        __fromCanvasComponent: true,
                        children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                          children: /* @__PURE__ */ _jsx5(motion4.p, {
                            style: {
                              '--framer-text-alignment': 'left',
                              '--framer-text-color': 'var(--extracted-r6o4lv, rgb(138, 138, 138))',
                            },
                            children: 'For flexible applications that have fluctuations in demand ',
                          },),
                        },),
                        className: 'framer-1cts9hh',
                        fonts: ['Inter',],
                        layoutDependency,
                        layoutId: 'LFBX3rguA',
                        style: { '--extracted-r6o4lv': 'rgb(138, 138, 138)', },
                        verticalAlignment: 'top',
                        withExternalLayout: true,
                      },),
                    ],
                  },),
                  /* @__PURE__ */ _jsxs4(motion4.div, {
                    className: 'framer-1q72eta',
                    layoutDependency,
                    layoutId: 'zh2ozQVzj',
                    children: [
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1xg4znl',
                        layoutDependency,
                        layoutId: 'ypWZxP4tr',
                        style: { backgroundColor: 'rgba(186, 221, 255, 0)', },
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1ug839q',
                        layoutDependency,
                        layoutId: 'eVM7qTUtt',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--font-selector': 'SW50ZXItQm9sZA==',
                                '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                '--framer-font-weight': '700',
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                              },
                              children: 'Community',
                            },),
                          },),
                          className: 'framer-yhha70',
                          fonts: ['Inter-Bold',],
                          layoutDependency,
                          layoutId: 'J5nd_GFE5',
                          style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-9qalg3',
                        layoutDependency,
                        layoutId: 'VzU3PQWGK',
                        style: { backgroundColor: 'rgba(255, 255, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--font-selector': 'SW50ZXItQm9sZA==',
                                '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                '--framer-font-weight': '700',
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                              },
                              children: 'Pro',
                            },),
                          },),
                          className: 'framer-14p0usc',
                          fonts: ['Inter-Bold',],
                          layoutDependency,
                          layoutId: 'Hmqd545TB',
                          style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1pwsib5',
                        layoutDependency,
                        layoutId: 'fKOYTD7tr',
                        style: { backgroundColor: 'rgba(186, 221, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--font-selector': 'SW50ZXItQm9sZA==',
                                '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                '--framer-font-weight': '700',
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                              },
                              children: 'Team',
                            },),
                          },),
                          className: 'framer-wdlvuy',
                          fonts: ['Inter-Bold',],
                          layoutDependency,
                          layoutId: 'rGr5qdmNl',
                          style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-5lpbh3',
                        layoutDependency,
                        layoutId: 'pMNd8_Cu7',
                        style: { backgroundColor: 'rgba(111, 136, 161, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--font-selector': 'SW50ZXItQm9sZA==',
                                '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                '--framer-font-weight': '700',
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                              },
                              children: 'Enterprise',
                            },),
                          },),
                          className: 'framer-1joqdpb',
                          fonts: ['Inter-Bold',],
                          layoutDependency,
                          layoutId: 'G8ntBIlEb',
                          style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(RichText4, {
                        __fromCanvasComponent: true,
                        children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                          children: /* @__PURE__ */ _jsx5(motion4.p, {
                            style: {
                              '--font-selector': 'R0Y7SW50ZXItNzAw',
                              '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                              '--framer-font-weight': '700',
                              '--framer-line-height': '1.5em',
                              '--framer-text-alignment': 'left',
                              '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                            },
                            children: 'Regions',
                          },),
                        },),
                        className: 'framer-1pvwsf4',
                        fonts: ['GF;Inter-700',],
                        layoutDependency,
                        layoutId: 'Qy9oVW_uv',
                        style: {
                          '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                          '--framer-link-text-color': 'rgb(0, 153, 255)',
                          '--framer-link-text-decoration': 'underline',
                          '--framer-paragraph-spacing': '0px',
                        },
                        verticalAlignment: 'center',
                        withExternalLayout: true,
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1d5vn6w',
                        layoutDependency,
                        layoutId: 'F2DSDfpFm',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                              },
                              children: '2 Regions',
                            },),
                          },),
                          className: 'framer-vdxi0',
                          fonts: ['Inter',],
                          layoutDependency,
                          layoutId: 'bXvt6bI1O',
                          style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1n66xs4',
                        layoutDependency,
                        layoutId: 'OjwpLEy8l',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                              },
                              children: '8  Regions',
                            },),
                          },),
                          className: 'framer-1wcks85',
                          fonts: ['Inter',],
                          layoutDependency,
                          layoutId: 'j7EbuKFnl',
                          style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-myuomn',
                        layoutDependency,
                        layoutId: 'P8ThFCVNa',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                              },
                              children: '8  Regions',
                            },),
                          },),
                          className: 'framer-9b0t6j',
                          fonts: ['Inter',],
                          layoutDependency,
                          layoutId: 'VLekI3gYu',
                          style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1905wxx',
                        layoutDependency,
                        layoutId: 'lToOHyvG0',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                              },
                              children: 'Custom',
                            },),
                          },),
                          className: 'framer-1t94q5v',
                          fonts: ['Inter',],
                          layoutDependency,
                          layoutId: 'g_I92H5c7',
                          style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(RichText4, {
                        __fromCanvasComponent: true,
                        children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                          children: /* @__PURE__ */ _jsx5(motion4.p, {
                            style: {
                              '--font-selector': 'R0Y7SW50ZXItNzAw',
                              '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                              '--framer-font-weight': '700',
                              '--framer-line-height': '1.5em',
                              '--framer-text-alignment': 'left',
                              '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                            },
                            children: 'Data Center Failover',
                          },),
                        },),
                        className: 'framer-nptynr',
                        fonts: ['GF;Inter-700',],
                        layoutDependency,
                        layoutId: 'ExTwBzxiS',
                        style: {
                          '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                          '--framer-link-text-color': 'rgb(0, 153, 255)',
                          '--framer-link-text-decoration': 'underline',
                          '--framer-paragraph-spacing': '0px',
                        },
                        verticalAlignment: 'center',
                        withExternalLayout: true,
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1xny62v',
                        layoutDependency,
                        layoutId: 'YL6RHiTHT',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                              },
                              children: '-',
                            },),
                          },),
                          className: 'framer-1hueufn',
                          fonts: ['Inter',],
                          layoutDependency,
                          layoutId: 'FuPbgawaB',
                          style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-lxtiiw',
                        layoutDependency,
                        layoutId: 'kPNsg2v51',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                              },
                              children: '-',
                            },),
                          },),
                          className: 'framer-zaam3s',
                          fonts: ['Inter',],
                          layoutDependency,
                          layoutId: 'MZtbJrupY',
                          style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-ag1lnz',
                        layoutDependency,
                        layoutId: 'g7LzSjYFW',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(SVG3, {
                          className: 'framer-188n9jw',
                          'data-framer-name': 'Check',
                          layout: 'position',
                          layoutDependency,
                          layoutId: 'sIZQoNgyT',
                          opacity: 1,
                          radius: 0,
                          style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                          svg:
                            '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 24 24"><path d="M 12 2.25 C 6.615 2.25 2.25 6.615 2.25 12 C 2.25 17.385 6.615 21.75 12 21.75 C 17.385 21.75 21.75 17.385 21.75 12 C 21.74 6.62 17.38 2.26 12 2.25 Z M 16.641 10.294 L 11.147 15.544 C 11.005 15.677 10.817 15.751 10.622 15.75 C 10.429 15.753 10.244 15.679 10.106 15.544 L 7.359 12.919 C 7.152 12.738 7.06 12.458 7.121 12.189 C 7.181 11.92 7.384 11.706 7.649 11.632 C 7.914 11.557 8.199 11.634 8.391 11.831 L 10.622 13.959 L 15.609 9.206 C 15.912 8.942 16.37 8.964 16.647 9.255 C 16.923 9.547 16.921 10.005 16.641 10.294 Z" fill="rgb(255, 79, 1)"></path></svg>',
                          svgContentId: 10285337634,
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-6gg80i',
                        layoutDependency,
                        layoutId: 'nFFsellnQ',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(SVG3, {
                          className: 'framer-1c9d5fk',
                          'data-framer-name': 'Check',
                          layout: 'position',
                          layoutDependency,
                          layoutId: 'yO7BlwkcL',
                          opacity: 1,
                          radius: 0,
                          style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                          svg:
                            '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 24 24"><path d="M 12 2.25 C 6.615 2.25 2.25 6.615 2.25 12 C 2.25 17.385 6.615 21.75 12 21.75 C 17.385 21.75 21.75 17.385 21.75 12 C 21.74 6.62 17.38 2.26 12 2.25 Z M 16.641 10.294 L 11.147 15.544 C 11.005 15.677 10.817 15.751 10.622 15.75 C 10.429 15.753 10.244 15.679 10.106 15.544 L 7.359 12.919 C 7.152 12.738 7.06 12.458 7.121 12.189 C 7.181 11.92 7.384 11.706 7.649 11.632 C 7.914 11.557 8.199 11.634 8.391 11.831 L 10.622 13.959 L 15.609 9.206 C 15.912 8.942 16.37 8.964 16.647 9.255 C 16.923 9.547 16.921 10.005 16.641 10.294 Z" fill="rgb(255, 79, 1)"></path></svg>',
                          svgContentId: 10285337634,
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(RichText4, {
                        __fromCanvasComponent: true,
                        children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                          children: /* @__PURE__ */ _jsx5(motion4.p, {
                            style: {
                              '--font-selector': 'R0Y7SW50ZXItNzAw',
                              '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                              '--framer-font-weight': '700',
                              '--framer-line-height': '1.5em',
                              '--framer-text-alignment': 'left',
                              '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                            },
                            children: 'Automatic SSL',
                          },),
                        },),
                        className: 'framer-mfhbn0',
                        fonts: ['GF;Inter-700',],
                        layoutDependency,
                        layoutId: 'srgjcvzQQ',
                        style: {
                          '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                          '--framer-link-text-color': 'rgb(0, 153, 255)',
                          '--framer-link-text-decoration': 'underline',
                          '--framer-paragraph-spacing': '0px',
                        },
                        verticalAlignment: 'center',
                        withExternalLayout: true,
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-wop075',
                        layoutDependency,
                        layoutId: 'MCYPck8eO',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                              },
                              children: '-',
                            },),
                          },),
                          className: 'framer-vtu735',
                          fonts: ['Inter',],
                          layoutDependency,
                          layoutId: 'KfGbqTxSk',
                          style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1qneiwc',
                        layoutDependency,
                        layoutId: 'QHcjwTIvp',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                              },
                              children: '-',
                            },),
                          },),
                          className: 'framer-18jtbce',
                          fonts: ['Inter',],
                          layoutDependency,
                          layoutId: 'pYUiW8GTm',
                          style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1g20fq2',
                        layoutDependency,
                        layoutId: 'AZpNp_s8l',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(SVG3, {
                          className: 'framer-e8wgrm',
                          'data-framer-name': 'Check',
                          layout: 'position',
                          layoutDependency,
                          layoutId: 'RQv35edUu',
                          opacity: 1,
                          radius: 0,
                          style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                          svg:
                            '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 24 24"><path d="M 12 2.25 C 6.615 2.25 2.25 6.615 2.25 12 C 2.25 17.385 6.615 21.75 12 21.75 C 17.385 21.75 21.75 17.385 21.75 12 C 21.74 6.62 17.38 2.26 12 2.25 Z M 16.641 10.294 L 11.147 15.544 C 11.005 15.677 10.817 15.751 10.622 15.75 C 10.429 15.753 10.244 15.679 10.106 15.544 L 7.359 12.919 C 7.152 12.738 7.06 12.458 7.121 12.189 C 7.181 11.92 7.384 11.706 7.649 11.632 C 7.914 11.557 8.199 11.634 8.391 11.831 L 10.622 13.959 L 15.609 9.206 C 15.912 8.942 16.37 8.964 16.647 9.255 C 16.923 9.547 16.921 10.005 16.641 10.294 Z" fill="rgb(255, 79, 1)"></path></svg>',
                          svgContentId: 10285337634,
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-186yg3s',
                        layoutDependency,
                        layoutId: 'iiiTaUDnI',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(SVG3, {
                          className: 'framer-14gmvoy',
                          'data-framer-name': 'Check',
                          layout: 'position',
                          layoutDependency,
                          layoutId: 'IFEOvf5d5',
                          opacity: 1,
                          radius: 0,
                          style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                          svg:
                            '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 24 24"><path d="M 12 2.25 C 6.615 2.25 2.25 6.615 2.25 12 C 2.25 17.385 6.615 21.75 12 21.75 C 17.385 21.75 21.75 17.385 21.75 12 C 21.74 6.62 17.38 2.26 12 2.25 Z M 16.641 10.294 L 11.147 15.544 C 11.005 15.677 10.817 15.751 10.622 15.75 C 10.429 15.753 10.244 15.679 10.106 15.544 L 7.359 12.919 C 7.152 12.738 7.06 12.458 7.121 12.189 C 7.181 11.92 7.384 11.706 7.649 11.632 C 7.914 11.557 8.199 11.634 8.391 11.831 L 10.622 13.959 L 15.609 9.206 C 15.912 8.942 16.37 8.964 16.647 9.255 C 16.923 9.547 16.921 10.005 16.641 10.294 Z" fill="rgb(255, 79, 1)"></path></svg>',
                          svgContentId: 10285337634,
                          withExternalLayout: true,
                        },),
                      },),
                    ],
                  },),
                ],
              },),
            },),
            isDisplayed() && /* @__PURE__ */ _jsx5(motion4.div, {
              className: 'framer-aww65b',
              layoutDependency,
              layoutId: 'gUMB2oYgr',
              children: /* @__PURE__ */ _jsxs4(motion4.div, {
                className: 'framer-ucfb0a',
                layoutDependency,
                layoutId: 'tJGdJKEqW',
                children: [
                  /* @__PURE__ */ _jsxs4(motion4.div, {
                    className: 'framer-np9h7e',
                    layoutDependency,
                    layoutId: 'gm7S5KppJ',
                    children: [
                      /* @__PURE__ */ _jsx5(RichText4, {
                        __fromCanvasComponent: true,
                        children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                          children: /* @__PURE__ */ _jsx5(motion4.h3, {
                            style: {
                              '--font-selector': 'SW50ZXItQm9sZA==',
                              '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                              '--framer-font-size': '24px',
                              '--framer-font-weight': '700',
                              '--framer-letter-spacing': '-1.9px',
                              '--framer-text-alignment': 'left',
                              '--framer-text-color': 'var(--extracted-a0htzi, rgb(255, 255, 255))',
                            },
                            children: 'Dedicated Hardware',
                          },),
                        },),
                        className: 'framer-a2l2lw',
                        fonts: ['Inter-Bold',],
                        layoutDependency,
                        layoutId: 'CADwWFuHp',
                        style: {
                          '--extracted-a0htzi': 'rgb(255, 255, 255)',
                          '--framer-link-text-color': 'rgb(0, 153, 255)',
                          '--framer-link-text-decoration': 'underline',
                          '--framer-paragraph-spacing': '0px',
                        },
                        verticalAlignment: 'top',
                        withExternalLayout: true,
                      },),
                      /* @__PURE__ */ _jsx5(RichText4, {
                        __fromCanvasComponent: true,
                        children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                          children: /* @__PURE__ */ _jsx5(motion4.p, {
                            style: {
                              '--framer-text-alignment': 'left',
                              '--framer-text-color': 'var(--extracted-r6o4lv, rgb(138, 138, 138))',
                            },
                            children: 'For applications that demand high performance',
                          },),
                        },),
                        className: 'framer-1qo3ksb',
                        fonts: ['Inter',],
                        layoutDependency,
                        layoutId: 'eJhPUQio6',
                        style: { '--extracted-r6o4lv': 'rgb(138, 138, 138)', },
                        verticalAlignment: 'top',
                        withExternalLayout: true,
                      },),
                    ],
                  },),
                  /* @__PURE__ */ _jsxs4(motion4.div, {
                    className: 'framer-f957eh',
                    layoutDependency,
                    layoutId: 'nyanw01o7',
                    children: [
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1pdqso7',
                        layoutDependency,
                        layoutId: 'DSJnYaKfc',
                        style: { backgroundColor: 'rgba(186, 221, 255, 0)', },
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-7v7ivb',
                        layoutDependency,
                        layoutId: 'ucJBu59U1',
                        style: { backgroundColor: 'rgba(46, 46, 46, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsxs4(React5.Fragment, {
                            children: [
                              /* @__PURE__ */ _jsx5(motion4.p, {
                                style: {
                                  '--font-selector': 'SW50ZXItQm9sZA==',
                                  '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                  '--framer-font-weight': '700',
                                  '--framer-text-alignment': 'center',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(87, 87, 87))',
                                },
                                children: 'Community ',
                              },),
                              /* @__PURE__ */ _jsx5(motion4.p, {
                                style: {
                                  '--font-selector': 'SW50ZXItQm9sZA==',
                                  '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                  '--framer-font-weight': '700',
                                  '--framer-text-alignment': 'center',
                                  '--framer-text-color': 'var(--extracted-2gxw0f, rgb(87, 87, 87))',
                                },
                                children: '(Not included)',
                              },),
                            ],
                          },),
                          className: 'framer-99tywc',
                          fonts: ['Inter-Bold',],
                          layoutDependency,
                          layoutId: 'zw1Dc9S_r',
                          style: { '--extracted-2gxw0f': 'rgb(87, 87, 87)', '--extracted-r6o4lv': 'rgb(87, 87, 87)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1a8pwtg',
                        layoutDependency,
                        layoutId: 'rBc_yDsjw',
                        style: { backgroundColor: 'rgba(255, 255, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--font-selector': 'SW50ZXItQm9sZA==',
                                '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                '--framer-font-weight': '700',
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                              },
                              children: 'Pro',
                            },),
                          },),
                          className: 'framer-oisjv1',
                          fonts: ['Inter-Bold',],
                          layoutDependency,
                          layoutId: 'IO8k2XFJz',
                          style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-7vbh0e',
                        layoutDependency,
                        layoutId: 'rrCkjjsdz',
                        style: { backgroundColor: 'rgba(186, 221, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--font-selector': 'SW50ZXItQm9sZA==',
                                '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                '--framer-font-weight': '700',
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                              },
                              children: 'Team',
                            },),
                          },),
                          className: 'framer-xw2o4k',
                          fonts: ['Inter-Bold',],
                          layoutDependency,
                          layoutId: 'Ky7lRDzyI',
                          style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1tcucf3',
                        layoutDependency,
                        layoutId: 'Vyi1_vmZB',
                        style: { backgroundColor: 'rgba(111, 136, 161, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--font-selector': 'SW50ZXItQm9sZA==',
                                '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                '--framer-font-weight': '700',
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                              },
                              children: 'Enterprise',
                            },),
                          },),
                          className: 'framer-1i5bydj',
                          fonts: ['Inter-Bold',],
                          layoutDependency,
                          layoutId: 'RSXrWBeBl',
                          style: { '--extracted-r6o4lv': 'rgb(255, 255, 255)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(RichText4, {
                        __fromCanvasComponent: true,
                        children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                          children: /* @__PURE__ */ _jsx5(motion4.p, {
                            style: {
                              '--font-selector': 'R0Y7SW50ZXItNzAw',
                              '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                              '--framer-font-weight': '700',
                              '--framer-line-height': '1.5em',
                              '--framer-text-alignment': 'left',
                              '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                            },
                            children: 'Any Region',
                          },),
                        },),
                        className: 'framer-lbq6cl',
                        fonts: ['GF;Inter-700',],
                        layoutDependency,
                        layoutId: 'LPfM7lZnL',
                        style: {
                          '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                          '--framer-link-text-color': 'rgb(0, 153, 255)',
                          '--framer-link-text-decoration': 'underline',
                          '--framer-paragraph-spacing': '0px',
                        },
                        verticalAlignment: 'center',
                        withExternalLayout: true,
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-gjmgpa',
                        layoutDependency,
                        layoutId: 'Y9orbHJyu',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(87, 87, 87))',
                              },
                              children: '-',
                            },),
                          },),
                          className: 'framer-1edbi3b',
                          fonts: ['Inter',],
                          layoutDependency,
                          layoutId: 'rbJURgU3B',
                          style: { '--extracted-r6o4lv': 'rgb(87, 87, 87)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1t4ojrk',
                        layoutDependency,
                        layoutId: 'i5nLx4Lsp',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(SVG3, {
                          className: 'framer-sj2229',
                          'data-framer-name': 'Check',
                          layout: 'position',
                          layoutDependency,
                          layoutId: 'UUkfVJQFO',
                          opacity: 1,
                          radius: 0,
                          style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                          svg:
                            '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 24 24"><path d="M 12 2.25 C 6.615 2.25 2.25 6.615 2.25 12 C 2.25 17.385 6.615 21.75 12 21.75 C 17.385 21.75 21.75 17.385 21.75 12 C 21.74 6.62 17.38 2.26 12 2.25 Z M 16.641 10.294 L 11.147 15.544 C 11.005 15.677 10.817 15.751 10.622 15.75 C 10.429 15.753 10.244 15.679 10.106 15.544 L 7.359 12.919 C 7.152 12.738 7.06 12.458 7.121 12.189 C 7.181 11.92 7.384 11.706 7.649 11.632 C 7.914 11.557 8.199 11.634 8.391 11.831 L 10.622 13.959 L 15.609 9.206 C 15.912 8.942 16.37 8.964 16.647 9.255 C 16.923 9.547 16.921 10.005 16.641 10.294 Z" fill="rgb(255, 79, 1)"></path></svg>',
                          svgContentId: 10285337634,
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1v71s83',
                        layoutDependency,
                        layoutId: 'Gb5Q9Kvm7',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(SVG3, {
                          className: 'framer-dbfywk',
                          'data-framer-name': 'Check',
                          layout: 'position',
                          layoutDependency,
                          layoutId: 'P0JmWQIu_',
                          opacity: 1,
                          radius: 0,
                          style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                          svg:
                            '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 24 24"><path d="M 12 2.25 C 6.615 2.25 2.25 6.615 2.25 12 C 2.25 17.385 6.615 21.75 12 21.75 C 17.385 21.75 21.75 17.385 21.75 12 C 21.74 6.62 17.38 2.26 12 2.25 Z M 16.641 10.294 L 11.147 15.544 C 11.005 15.677 10.817 15.751 10.622 15.75 C 10.429 15.753 10.244 15.679 10.106 15.544 L 7.359 12.919 C 7.152 12.738 7.06 12.458 7.121 12.189 C 7.181 11.92 7.384 11.706 7.649 11.632 C 7.914 11.557 8.199 11.634 8.391 11.831 L 10.622 13.959 L 15.609 9.206 C 15.912 8.942 16.37 8.964 16.647 9.255 C 16.923 9.547 16.921 10.005 16.641 10.294 Z" fill="rgb(255, 79, 1)"></path></svg>',
                          svgContentId: 10285337634,
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1r888j2',
                        layoutDependency,
                        layoutId: 'AD1Ax2J2e',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(SVG3, {
                          className: 'framer-dr5idc',
                          'data-framer-name': 'Check',
                          layout: 'position',
                          layoutDependency,
                          layoutId: 'yba0Etetq',
                          opacity: 1,
                          radius: 0,
                          style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                          svg:
                            '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 24 24"><path d="M 12 2.25 C 6.615 2.25 2.25 6.615 2.25 12 C 2.25 17.385 6.615 21.75 12 21.75 C 17.385 21.75 21.75 17.385 21.75 12 C 21.74 6.62 17.38 2.26 12 2.25 Z M 16.641 10.294 L 11.147 15.544 C 11.005 15.677 10.817 15.751 10.622 15.75 C 10.429 15.753 10.244 15.679 10.106 15.544 L 7.359 12.919 C 7.152 12.738 7.06 12.458 7.121 12.189 C 7.181 11.92 7.384 11.706 7.649 11.632 C 7.914 11.557 8.199 11.634 8.391 11.831 L 10.622 13.959 L 15.609 9.206 C 15.912 8.942 16.37 8.964 16.647 9.255 C 16.923 9.547 16.921 10.005 16.641 10.294 Z" fill="rgb(255, 79, 1)"></path></svg>',
                          svgContentId: 10285337634,
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(RichText4, {
                        __fromCanvasComponent: true,
                        children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                          children: /* @__PURE__ */ _jsx5(motion4.p, {
                            style: {
                              '--font-selector': 'R0Y7SW50ZXItNzAw',
                              '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                              '--framer-font-weight': '700',
                              '--framer-line-height': '1.5em',
                              '--framer-text-alignment': 'left',
                              '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                            },
                            children: 'Providers',
                          },),
                        },),
                        className: 'framer-11g8lak',
                        fonts: ['GF;Inter-700',],
                        layoutDependency,
                        layoutId: 'pa6agYSW5',
                        style: {
                          '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                          '--framer-link-text-color': 'rgb(0, 153, 255)',
                          '--framer-link-text-decoration': 'underline',
                          '--framer-paragraph-spacing': '0px',
                        },
                        verticalAlignment: 'center',
                        withExternalLayout: true,
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-smhpgc',
                        layoutDependency,
                        layoutId: 'boSyHMOqm',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(87, 87, 87))',
                              },
                              children: '-',
                            },),
                          },),
                          className: 'framer-1lq38p9',
                          fonts: ['Inter',],
                          layoutDependency,
                          layoutId: 'KKuMYgSWv',
                          style: { '--extracted-r6o4lv': 'rgb(87, 87, 87)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-184xs2b',
                        layoutDependency,
                        layoutId: 'eHck7C0Zn',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsxs4(React5.Fragment, {
                            children: [
                              /* @__PURE__ */ _jsx5(motion4.p, {
                                style: {
                                  '--framer-text-alignment': 'center',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(133, 133, 133))',
                                },
                                children: 'Linode',
                              },),
                              /* @__PURE__ */ _jsx5(motion4.p, {
                                style: {
                                  '--framer-text-alignment': 'center',
                                  '--framer-text-color': 'var(--extracted-2gxw0f, rgb(133, 133, 133))',
                                },
                                children: ' (more coming soon)',
                              },),
                            ],
                          },),
                          className: 'framer-yi42xo',
                          fonts: ['Inter',],
                          layoutDependency,
                          layoutId: 'zWEizHL9d',
                          style: { '--extracted-2gxw0f': 'rgb(133, 133, 133)', '--extracted-r6o4lv': 'rgb(133, 133, 133)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-ee8a2w',
                        layoutDependency,
                        layoutId: 'yy9RdA2x2',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsxs4(React5.Fragment, {
                            children: [
                              /* @__PURE__ */ _jsx5(motion4.p, {
                                style: {
                                  '--framer-text-alignment': 'center',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(133, 133, 133))',
                                },
                                children: 'Linode ',
                              },),
                              /* @__PURE__ */ _jsx5(motion4.p, {
                                style: {
                                  '--framer-text-alignment': 'center',
                                  '--framer-text-color': 'var(--extracted-2gxw0f, rgb(133, 133, 133))',
                                },
                                children: '(more coming soon)',
                              },),
                            ],
                          },),
                          className: 'framer-qbpn7v',
                          fonts: ['Inter',],
                          layoutDependency,
                          layoutId: 'm76NQOKyK',
                          style: { '--extracted-2gxw0f': 'rgb(133, 133, 133)', '--extracted-r6o4lv': 'rgb(133, 133, 133)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1v4ghzi',
                        layoutDependency,
                        layoutId: 'dK9mPSwgT',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(133, 133, 133))',
                              },
                              children: 'Custom',
                            },),
                          },),
                          className: 'framer-5w5ipt',
                          fonts: ['Inter',],
                          layoutDependency,
                          layoutId: 'yII3oPXmp',
                          style: { '--extracted-r6o4lv': 'rgb(133, 133, 133)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(RichText4, {
                        __fromCanvasComponent: true,
                        children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                          children: /* @__PURE__ */ _jsx5(motion4.p, {
                            style: {
                              '--font-selector': 'R0Y7SW50ZXItNzAw',
                              '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                              '--framer-font-weight': '700',
                              '--framer-line-height': '1.5em',
                              '--framer-text-alignment': 'left',
                              '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                            },
                            children: 'Data Center Failover',
                          },),
                        },),
                        className: 'framer-1vn6djv',
                        fonts: ['GF;Inter-700',],
                        layoutDependency,
                        layoutId: 'LFsq_VhQH',
                        style: {
                          '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                          '--framer-link-text-color': 'rgb(0, 153, 255)',
                          '--framer-link-text-decoration': 'underline',
                          '--framer-paragraph-spacing': '0px',
                        },
                        verticalAlignment: 'center',
                        withExternalLayout: true,
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1x73oz0',
                        layoutDependency,
                        layoutId: 'N6nQkQrZO',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(87, 87, 87))',
                              },
                              children: '-',
                            },),
                          },),
                          className: 'framer-1v0nrzs',
                          fonts: ['Inter',],
                          layoutDependency,
                          layoutId: 'JHkGlbE93',
                          style: { '--extracted-r6o4lv': 'rgb(87, 87, 87)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-13yhtwp',
                        layoutDependency,
                        layoutId: 'KEx8pLcaA',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(SVG3, {
                          className: 'framer-1frdxxx',
                          'data-framer-name': 'Check',
                          layout: 'position',
                          layoutDependency,
                          layoutId: 'ooPfOorvy',
                          opacity: 1,
                          radius: 0,
                          style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                          svg:
                            '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 24 24"><path d="M 12 2.25 C 6.615 2.25 2.25 6.615 2.25 12 C 2.25 17.385 6.615 21.75 12 21.75 C 17.385 21.75 21.75 17.385 21.75 12 C 21.74 6.62 17.38 2.26 12 2.25 Z M 16.641 10.294 L 11.147 15.544 C 11.005 15.677 10.817 15.751 10.622 15.75 C 10.429 15.753 10.244 15.679 10.106 15.544 L 7.359 12.919 C 7.152 12.738 7.06 12.458 7.121 12.189 C 7.181 11.92 7.384 11.706 7.649 11.632 C 7.914 11.557 8.199 11.634 8.391 11.831 L 10.622 13.959 L 15.609 9.206 C 15.912 8.942 16.37 8.964 16.647 9.255 C 16.923 9.547 16.921 10.005 16.641 10.294 Z" fill="rgb(255, 79, 1)"></path></svg>',
                          svgContentId: 10285337634,
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-m8nrn5',
                        layoutDependency,
                        layoutId: 'xafnnfZKv',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(SVG3, {
                          className: 'framer-1ph6jbd',
                          'data-framer-name': 'Check',
                          layout: 'position',
                          layoutDependency,
                          layoutId: 'ljBj8GTqD',
                          opacity: 1,
                          radius: 0,
                          style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                          svg:
                            '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 24 24"><path d="M 12 2.25 C 6.615 2.25 2.25 6.615 2.25 12 C 2.25 17.385 6.615 21.75 12 21.75 C 17.385 21.75 21.75 17.385 21.75 12 C 21.74 6.62 17.38 2.26 12 2.25 Z M 16.641 10.294 L 11.147 15.544 C 11.005 15.677 10.817 15.751 10.622 15.75 C 10.429 15.753 10.244 15.679 10.106 15.544 L 7.359 12.919 C 7.152 12.738 7.06 12.458 7.121 12.189 C 7.181 11.92 7.384 11.706 7.649 11.632 C 7.914 11.557 8.199 11.634 8.391 11.831 L 10.622 13.959 L 15.609 9.206 C 15.912 8.942 16.37 8.964 16.647 9.255 C 16.923 9.547 16.921 10.005 16.641 10.294 Z" fill="rgb(255, 79, 1)"></path></svg>',
                          svgContentId: 10285337634,
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1xupaty',
                        layoutDependency,
                        layoutId: 'zJKKirWIu',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(SVG3, {
                          className: 'framer-1uc2uza',
                          'data-framer-name': 'Check',
                          layout: 'position',
                          layoutDependency,
                          layoutId: 'aM5EkBIWB',
                          opacity: 1,
                          radius: 0,
                          style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                          svg:
                            '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 24 24"><path d="M 12 2.25 C 6.615 2.25 2.25 6.615 2.25 12 C 2.25 17.385 6.615 21.75 12 21.75 C 17.385 21.75 21.75 17.385 21.75 12 C 21.74 6.62 17.38 2.26 12 2.25 Z M 16.641 10.294 L 11.147 15.544 C 11.005 15.677 10.817 15.751 10.622 15.75 C 10.429 15.753 10.244 15.679 10.106 15.544 L 7.359 12.919 C 7.152 12.738 7.06 12.458 7.121 12.189 C 7.181 11.92 7.384 11.706 7.649 11.632 C 7.914 11.557 8.199 11.634 8.391 11.831 L 10.622 13.959 L 15.609 9.206 C 15.912 8.942 16.37 8.964 16.647 9.255 C 16.923 9.547 16.921 10.005 16.641 10.294 Z" fill="rgb(255, 79, 1)"></path></svg>',
                          svgContentId: 10285337634,
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(RichText4, {
                        __fromCanvasComponent: true,
                        children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                          children: /* @__PURE__ */ _jsx5(motion4.p, {
                            style: {
                              '--font-selector': 'R0Y7SW50ZXItNzAw',
                              '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                              '--framer-font-weight': '700',
                              '--framer-line-height': '1.5em',
                              '--framer-text-alignment': 'left',
                              '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                            },
                            children: 'Automatic SSL',
                          },),
                        },),
                        className: 'framer-bql8xh',
                        fonts: ['GF;Inter-700',],
                        layoutDependency,
                        layoutId: 'MHj_onS30',
                        style: {
                          '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                          '--framer-link-text-color': 'rgb(0, 153, 255)',
                          '--framer-link-text-decoration': 'underline',
                          '--framer-paragraph-spacing': '0px',
                        },
                        verticalAlignment: 'center',
                        withExternalLayout: true,
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-1hlpdp3',
                        layoutDependency,
                        layoutId: 'LKeiCeZHJ',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(RichText4, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                            children: /* @__PURE__ */ _jsx5(motion4.p, {
                              style: {
                                '--framer-text-alignment': 'center',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(87, 87, 87))',
                              },
                              children: '-',
                            },),
                          },),
                          className: 'framer-tfpd6e',
                          fonts: ['Inter',],
                          layoutDependency,
                          layoutId: 'ZPDjwlmtF',
                          style: { '--extracted-r6o4lv': 'rgb(87, 87, 87)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-bxbveo',
                        layoutDependency,
                        layoutId: 'OelbqfNS0',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(SVG3, {
                          className: 'framer-ngueq9',
                          'data-framer-name': 'Check',
                          layout: 'position',
                          layoutDependency,
                          layoutId: 'AJRNVM4mp',
                          opacity: 1,
                          radius: 0,
                          style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                          svg:
                            '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 24 24"><path d="M 12 2.25 C 6.615 2.25 2.25 6.615 2.25 12 C 2.25 17.385 6.615 21.75 12 21.75 C 17.385 21.75 21.75 17.385 21.75 12 C 21.74 6.62 17.38 2.26 12 2.25 Z M 16.641 10.294 L 11.147 15.544 C 11.005 15.677 10.817 15.751 10.622 15.75 C 10.429 15.753 10.244 15.679 10.106 15.544 L 7.359 12.919 C 7.152 12.738 7.06 12.458 7.121 12.189 C 7.181 11.92 7.384 11.706 7.649 11.632 C 7.914 11.557 8.199 11.634 8.391 11.831 L 10.622 13.959 L 15.609 9.206 C 15.912 8.942 16.37 8.964 16.647 9.255 C 16.923 9.547 16.921 10.005 16.641 10.294 Z" fill="rgb(255, 79, 1)"></path></svg>',
                          svgContentId: 10285337634,
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-f79owq',
                        layoutDependency,
                        layoutId: 'gUCiulrjV',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(SVG3, {
                          className: 'framer-ua9zck',
                          'data-framer-name': 'Check',
                          layout: 'position',
                          layoutDependency,
                          layoutId: 'aZPpJHWJz',
                          opacity: 1,
                          radius: 0,
                          style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                          svg:
                            '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 24 24"><path d="M 12 2.25 C 6.615 2.25 2.25 6.615 2.25 12 C 2.25 17.385 6.615 21.75 12 21.75 C 17.385 21.75 21.75 17.385 21.75 12 C 21.74 6.62 17.38 2.26 12 2.25 Z M 16.641 10.294 L 11.147 15.544 C 11.005 15.677 10.817 15.751 10.622 15.75 C 10.429 15.753 10.244 15.679 10.106 15.544 L 7.359 12.919 C 7.152 12.738 7.06 12.458 7.121 12.189 C 7.181 11.92 7.384 11.706 7.649 11.632 C 7.914 11.557 8.199 11.634 8.391 11.831 L 10.622 13.959 L 15.609 9.206 C 15.912 8.942 16.37 8.964 16.647 9.255 C 16.923 9.547 16.921 10.005 16.641 10.294 Z" fill="rgb(255, 79, 1)"></path></svg>',
                          svgContentId: 10285337634,
                          withExternalLayout: true,
                        },),
                      },),
                      /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-10gvatr',
                        layoutDependency,
                        layoutId: 'CFwNIPOaw',
                        style: { backgroundColor: 'rgba(204, 238, 255, 0)', },
                        children: /* @__PURE__ */ _jsx5(SVG3, {
                          className: 'framer-qwcsss',
                          'data-framer-name': 'Check',
                          layout: 'position',
                          layoutDependency,
                          layoutId: 'qoQHhc5SX',
                          opacity: 1,
                          radius: 0,
                          style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                          svg:
                            '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 24 24"><path d="M 12 2.25 C 6.615 2.25 2.25 6.615 2.25 12 C 2.25 17.385 6.615 21.75 12 21.75 C 17.385 21.75 21.75 17.385 21.75 12 C 21.74 6.62 17.38 2.26 12 2.25 Z M 16.641 10.294 L 11.147 15.544 C 11.005 15.677 10.817 15.751 10.622 15.75 C 10.429 15.753 10.244 15.679 10.106 15.544 L 7.359 12.919 C 7.152 12.738 7.06 12.458 7.121 12.189 C 7.181 11.92 7.384 11.706 7.649 11.632 C 7.914 11.557 8.199 11.634 8.391 11.831 L 10.622 13.959 L 15.609 9.206 C 15.912 8.942 16.37 8.964 16.647 9.255 C 16.923 9.547 16.921 10.005 16.641 10.294 Z" fill="rgb(255, 79, 1)"></path></svg>',
                          svgContentId: 10285337634,
                          withExternalLayout: true,
                        },),
                      },),
                    ],
                  },),
                ],
              },),
            },),
            isDisplayed1() && /* @__PURE__ */ _jsxs4(motion4.section, {
              className: 'framer-8f6pe6',
              'data-framer-name': 'Phone',
              layoutDependency,
              layoutId: 'lFvgqcLcF',
              style: { backgroundColor: 'rgb(0, 0, 0)', },
              children: [
                /* @__PURE__ */ _jsxs4(motion4.div, {
                  className: 'framer-tlmrr2',
                  layoutDependency,
                  layoutId: 'PCJDmjCcW',
                  children: [
                    /* @__PURE__ */ _jsx5(RichText4, {
                      __fromCanvasComponent: true,
                      children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                        children: /* @__PURE__ */ _jsx5(motion4.h3, {
                          className: 'framer-styles-preset-jttjmp',
                          'data-styles-preset': 'zu841OiIg',
                          style: { '--framer-text-alignment': 'center', },
                          children: 'Rivet Cloud Pricing',
                        },),
                      },),
                      className: 'framer-1hbajtu',
                      fonts: ['Inter',],
                      layoutDependency,
                      layoutId: 'R4SHX3WE7',
                      style: {
                        '--framer-link-text-color': 'rgb(0, 153, 255)',
                        '--framer-link-text-decoration': 'underline',
                        '--framer-paragraph-spacing': '0px',
                      },
                      verticalAlignment: 'top',
                      withExternalLayout: true,
                    },),
                    /* @__PURE__ */ _jsx5(RichText4, {
                      __fromCanvasComponent: true,
                      children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                        children: /* @__PURE__ */ _jsx5(motion4.h2, {
                          className: 'framer-styles-preset-az499w',
                          'data-styles-preset': 'kHb0JRZSH',
                          style: { '--framer-text-alignment': 'center', },
                          children: 'Start for free with scalable pricing as you grow.',
                        },),
                      },),
                      className: 'framer-9qs87f',
                      fonts: ['Inter',],
                      layoutDependency,
                      layoutId: 'CeXF7dLuQ',
                      style: { '--framer-paragraph-spacing': '0px', },
                      verticalAlignment: 'top',
                      withExternalLayout: true,
                    },),
                  ],
                },),
                /* @__PURE__ */ _jsx5(motion4.div, {
                  className: 'framer-4q8dy1',
                  layoutDependency,
                  layoutId: 'UwFPfRDlt',
                  children: /* @__PURE__ */ _jsxs4(motion4.div, {
                    className: 'framer-1i52f4v',
                    'data-framer-name': 'Container',
                    layoutDependency,
                    layoutId: 'EmTvlfarb',
                    children: [
                      /* @__PURE__ */ _jsxs4(motion4.div, {
                        className: 'framer-mimhod',
                        'data-border': true,
                        'data-framer-name': 'Card',
                        layoutDependency,
                        layoutId: 'P_QIPDarI',
                        style: {
                          '--border-bottom-width': '1px',
                          '--border-color': 'rgb(77, 77, 77)',
                          '--border-left-width': '1px',
                          '--border-right-width': '1px',
                          '--border-style': 'solid',
                          '--border-top-width': '1px',
                          backgroundColor: 'rgb(8, 8, 8)',
                          borderBottomLeftRadius: 6,
                          borderBottomRightRadius: 6,
                          borderTopLeftRadius: 6,
                          borderTopRightRadius: 6,
                          boxShadow:
                            '0px 0.7961918735236395px 2.3885756205709185px -0.625px rgba(0, 0, 0, 0.05), 0px 2.414506143104518px 7.2435184293135535px -1.25px rgba(0, 0, 0, 0.05), 0px 6.382653521484461px 19.147960564453385px -1.875px rgba(0, 0, 0, 0.05), 0px 20px 60px -2.5px rgba(0, 0, 0, 0.05)',
                        },
                        children: [
                          /* @__PURE__ */ _jsx5(RichText4, {
                            __fromCanvasComponent: true,
                            children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                              children: /* @__PURE__ */ _jsx5(motion4.p, {
                                style: {
                                  '--font-selector': 'R0Y7SW50ZXItNjAw',
                                  '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                  '--framer-font-size': '24px',
                                  '--framer-font-weight': '600',
                                  '--framer-line-height': '1.5em',
                                  '--framer-text-alignment': 'left',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                                },
                                children: 'Community',
                              },),
                            },),
                            className: 'framer-1y1o55a',
                            fonts: ['GF;Inter-600',],
                            layoutDependency,
                            layoutId: 'nwvC80Jld',
                            style: {
                              '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                              '--framer-link-text-color': 'rgb(0, 153, 255)',
                              '--framer-link-text-decoration': 'underline',
                              '--framer-paragraph-spacing': '0px',
                            },
                            verticalAlignment: 'top',
                            withExternalLayout: true,
                          },),
                          /* @__PURE__ */ _jsxs4(motion4.div, {
                            className: 'framer-1r68b23',
                            layoutDependency,
                            layoutId: 'uhntIkSsy',
                            children: [
                              /* @__PURE__ */ _jsx5(RichText4, {
                                __fromCanvasComponent: true,
                                children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                  children: /* @__PURE__ */ _jsx5(motion4.p, {
                                    style: {
                                      '--font-selector': 'R0Y7SW50ZXItNzAw',
                                      '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                      '--framer-font-size': '40px',
                                      '--framer-font-weight': '700',
                                      '--framer-letter-spacing': '-3px',
                                      '--framer-line-height': '1em',
                                      '--framer-text-alignment': 'left',
                                      '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                                    },
                                    children: '$0',
                                  },),
                                },),
                                className: 'framer-99vqpv',
                                fonts: ['GF;Inter-700',],
                                layoutDependency,
                                layoutId: 'LHm6nBe57',
                                style: {
                                  '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                                  '--framer-link-text-color': 'rgb(0, 153, 255)',
                                  '--framer-link-text-decoration': 'underline',
                                  '--framer-paragraph-spacing': '0px',
                                },
                                verticalAlignment: 'top',
                                withExternalLayout: true,
                              },),
                              /* @__PURE__ */ _jsx5(RichText4, {
                                __fromCanvasComponent: true,
                                children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                  children: /* @__PURE__ */ _jsx5(motion4.p, {
                                    style: {
                                      '--font-selector': 'R0Y7SW50ZXItNjAw',
                                      '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                      '--framer-font-size': '14px',
                                      '--framer-font-weight': '600',
                                      '--framer-line-height': '1.5em',
                                      '--framer-text-alignment': 'left',
                                      '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                    },
                                    children: '/month',
                                  },),
                                },),
                                className: 'framer-1g8j7hc',
                                fonts: ['GF;Inter-600',],
                                layoutDependency,
                                layoutId: 'yBjpIFkUb',
                                style: {
                                  '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                  '--framer-link-text-color': 'rgb(0, 153, 255)',
                                  '--framer-link-text-decoration': 'underline',
                                  '--framer-paragraph-spacing': '0px',
                                },
                                verticalAlignment: 'top',
                                withExternalLayout: true,
                              },),
                            ],
                          },),
                          /* @__PURE__ */ _jsx5(RichText4, {
                            __fromCanvasComponent: true,
                            children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                              children: /* @__PURE__ */ _jsx5(motion4.p, {
                                style: {
                                  '--font-selector': 'R0Y7SW50ZXItNjAw',
                                  '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                  '--framer-font-size': '14px',
                                  '--framer-font-weight': '600',
                                  '--framer-line-height': '1.5em',
                                  '--framer-text-alignment': 'left',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                },
                                children: '+Resource Usage',
                              },),
                            },),
                            className: 'framer-170zt9',
                            fonts: ['GF;Inter-600',],
                            layoutDependency,
                            layoutId: 'aUAt4kVcM',
                            style: {
                              '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                              '--framer-link-text-color': 'rgb(0, 153, 255)',
                              '--framer-link-text-decoration': 'underline',
                              '--framer-paragraph-spacing': '0px',
                            },
                            verticalAlignment: 'top',
                            withExternalLayout: true,
                          },),
                          /* @__PURE__ */ _jsxs4(motion4.div, {
                            className: 'framer-1h3ybkb',
                            layoutDependency,
                            layoutId: 'YHY_ClYsd',
                            children: [
                              /* @__PURE__ */ _jsxs4(motion4.div, {
                                className: 'framer-1vfcu4x',
                                'data-framer-name': 'Row',
                                layoutDependency,
                                layoutId: 'WJM08f1F1',
                                children: [
                                  /* @__PURE__ */ _jsx5(ComponentViewportProvider, {
                                    children: /* @__PURE__ */ _jsx5(motion4.div, {
                                      className: 'framer-rsyofj-container',
                                      layoutDependency,
                                      layoutId: 'zvZfI9iHX-container',
                                      children: /* @__PURE__ */ _jsx5(Icon, {
                                        color: 'rgb(255, 255, 255)',
                                        height: '100%',
                                        iconSearch: 'Home',
                                        iconSelection: 'gift',
                                        id: 'zvZfI9iHX',
                                        layoutId: 'zvZfI9iHX',
                                        mirrored: false,
                                        selectByList: true,
                                        style: { height: '100%', width: '100%', },
                                        width: '100%',
                                      },),
                                    },),
                                  },),
                                  /* @__PURE__ */ _jsx5(RichText4, {
                                    __fromCanvasComponent: true,
                                    children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                      children: /* @__PURE__ */ _jsx5(motion4.p, {
                                        style: {
                                          '--font-selector': 'R0Y7SW50ZXItNjAw',
                                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                          '--framer-font-size': '12px',
                                          '--framer-font-weight': '600',
                                          '--framer-line-height': '1.5em',
                                          '--framer-text-alignment': 'left',
                                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                        },
                                        children: '$5 monthly credit',
                                      },),
                                    },),
                                    className: 'framer-tvg0jw',
                                    fonts: ['GF;Inter-600',],
                                    layoutDependency,
                                    layoutId: 'oA2qyCJUw',
                                    style: {
                                      '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                      '--framer-link-text-color': 'rgb(0, 153, 255)',
                                      '--framer-link-text-decoration': 'underline',
                                      '--framer-paragraph-spacing': '0px',
                                    },
                                    verticalAlignment: 'top',
                                    withExternalLayout: true,
                                  },),
                                ],
                              },),
                              /* @__PURE__ */ _jsxs4(motion4.div, {
                                className: 'framer-14260yl',
                                'data-framer-name': 'Row',
                                layoutDependency,
                                layoutId: 'v0BmnbLzq',
                                children: [
                                  /* @__PURE__ */ _jsx5(SVG3, {
                                    className: 'framer-12q19vw',
                                    'data-framer-name': 'Check',
                                    layout: 'position',
                                    layoutDependency,
                                    layoutId: 'Stn1fsveg',
                                    opacity: 1,
                                    radius: 0,
                                    style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                    svg:
                                      '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                    svgContentId: 10075247493,
                                    withExternalLayout: true,
                                  },),
                                  /* @__PURE__ */ _jsx5(RichText4, {
                                    __fromCanvasComponent: true,
                                    children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                      children: /* @__PURE__ */ _jsx5(motion4.p, {
                                        style: {
                                          '--font-selector': 'R0Y7SW50ZXItNjAw',
                                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                          '--framer-font-size': '12px',
                                          '--framer-font-weight': '600',
                                          '--framer-line-height': '1.5em',
                                          '--framer-text-alignment': 'left',
                                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                        },
                                        children: 'DDoS Mitigation',
                                      },),
                                    },),
                                    className: 'framer-195sdz5',
                                    fonts: ['GF;Inter-600',],
                                    layoutDependency,
                                    layoutId: 'BqyXAfSe9',
                                    style: {
                                      '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                      '--framer-link-text-color': 'rgb(0, 153, 255)',
                                      '--framer-link-text-decoration': 'underline',
                                      '--framer-paragraph-spacing': '0px',
                                    },
                                    verticalAlignment: 'top',
                                    withExternalLayout: true,
                                  },),
                                ],
                              },),
                              /* @__PURE__ */ _jsxs4(motion4.div, {
                                className: 'framer-1kx0z0g',
                                'data-framer-name': 'Row',
                                layoutDependency,
                                layoutId: 'YpyVRhVSr',
                                children: [
                                  /* @__PURE__ */ _jsx5(SVG3, {
                                    className: 'framer-t0l3i6',
                                    'data-framer-name': 'Check',
                                    layout: 'position',
                                    layoutDependency,
                                    layoutId: 'QxNZru5gT',
                                    opacity: 1,
                                    radius: 0,
                                    style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                    svg:
                                      '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                    svgContentId: 10075247493,
                                    withExternalLayout: true,
                                  },),
                                  /* @__PURE__ */ _jsx5(RichText4, {
                                    __fromCanvasComponent: true,
                                    children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                      children: /* @__PURE__ */ _jsx5(motion4.p, {
                                        style: {
                                          '--font-selector': 'R0Y7SW50ZXItNjAw',
                                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                          '--framer-font-size': '12px',
                                          '--framer-font-weight': '600',
                                          '--framer-line-height': '1.5em',
                                          '--framer-text-alignment': 'left',
                                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                        },
                                        children: 'SSL Management ',
                                      },),
                                    },),
                                    className: 'framer-171k1td',
                                    fonts: ['GF;Inter-600',],
                                    layoutDependency,
                                    layoutId: 'J9zP913J5',
                                    style: {
                                      '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                      '--framer-link-text-color': 'rgb(0, 153, 255)',
                                      '--framer-link-text-decoration': 'underline',
                                      '--framer-paragraph-spacing': '0px',
                                    },
                                    verticalAlignment: 'top',
                                    withExternalLayout: true,
                                  },),
                                ],
                              },),
                              /* @__PURE__ */ _jsxs4(motion4.div, {
                                className: 'framer-1lemtug',
                                'data-framer-name': 'Row',
                                layoutDependency,
                                layoutId: 'qIrFAQEzm',
                                children: [
                                  /* @__PURE__ */ _jsx5(SVG3, {
                                    className: 'framer-1v34jr2',
                                    'data-framer-name': 'Check',
                                    layout: 'position',
                                    layoutDependency,
                                    layoutId: 'we6gfKWC9',
                                    opacity: 1,
                                    radius: 0,
                                    style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                    svg:
                                      '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                    svgContentId: 10075247493,
                                    withExternalLayout: true,
                                  },),
                                  /* @__PURE__ */ _jsx5(RichText4, {
                                    __fromCanvasComponent: true,
                                    children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                      children: /* @__PURE__ */ _jsx5(motion4.p, {
                                        style: {
                                          '--font-selector': 'R0Y7SW50ZXItNjAw',
                                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                          '--framer-font-size': '12px',
                                          '--framer-font-weight': '600',
                                          '--framer-line-height': '1.5em',
                                          '--framer-text-alignment': 'left',
                                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                        },
                                        children: 'Community Support',
                                      },),
                                    },),
                                    className: 'framer-1his063',
                                    fonts: ['GF;Inter-600',],
                                    layoutDependency,
                                    layoutId: 'uDDMYmBjC',
                                    style: {
                                      '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                      '--framer-link-text-color': 'rgb(0, 153, 255)',
                                      '--framer-link-text-decoration': 'underline',
                                      '--framer-paragraph-spacing': '0px',
                                    },
                                    verticalAlignment: 'top',
                                    withExternalLayout: true,
                                  },),
                                ],
                              },),
                            ],
                          },),
                          /* @__PURE__ */ _jsx5(motion4.div, {
                            className: 'framer-o9sfrn',
                            'data-framer-name': 'Button',
                            layoutDependency,
                            layoutId: 'nY2Uhmadl',
                            style: {
                              backgroundColor: 'rgb(255, 79, 0)',
                              borderBottomLeftRadius: 4,
                              borderBottomRightRadius: 4,
                              borderTopLeftRadius: 4,
                              borderTopRightRadius: 4,
                              boxShadow:
                                '0px 0.7065919983928324px 0.7065919983928324px -0.625px rgba(0, 0, 0, 0.14764), 0px 1.8065619053231785px 1.8065619053231785px -1.25px rgba(0, 0, 0, 0.14398), 0px 3.6217592146567767px 3.6217592146567767px -1.875px rgba(0, 0, 0, 0.13793), 0px 6.8655999097303715px 6.8655999097303715px -2.5px rgba(0, 0, 0, 0.12711), 0px 13.646761411524492px 13.646761411524492px -3.125px rgba(0, 0, 0, 0.10451), 0px 30px 30px -3.75px rgba(0, 0, 0, 0.05)',
                            },
                            children: /* @__PURE__ */ _jsx5(RichText4, {
                              __fromCanvasComponent: true,
                              children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                children: /* @__PURE__ */ _jsx5(motion4.p, {
                                  style: {
                                    '--font-selector': 'R0Y7SW50ZXItNjAw',
                                    '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                    '--framer-font-size': '14px',
                                    '--framer-font-weight': '600',
                                    '--framer-letter-spacing': '0px',
                                    '--framer-text-alignment': 'center',
                                    '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                                  },
                                  children: 'Get Started',
                                },),
                              },),
                              className: 'framer-tpbss6',
                              fonts: ['GF;Inter-600',],
                              layoutDependency,
                              layoutId: 'HNj7q3aCF',
                              style: {
                                '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                                '--framer-link-text-color': 'rgb(0, 153, 255)',
                                '--framer-link-text-decoration': 'underline',
                                '--framer-paragraph-spacing': '0px',
                              },
                              verticalAlignment: 'top',
                              withExternalLayout: true,
                            },),
                          },),
                        ],
                      },),
                      /* @__PURE__ */ _jsxs4(motion4.div, {
                        className: 'framer-h4jq97',
                        'data-border': true,
                        'data-framer-name': 'Card',
                        layoutDependency,
                        layoutId: 'ap60WWa_y',
                        style: {
                          '--border-bottom-width': '1px',
                          '--border-color': 'rgb(77, 77, 77)',
                          '--border-left-width': '1px',
                          '--border-right-width': '1px',
                          '--border-style': 'solid',
                          '--border-top-width': '1px',
                          backgroundColor: 'rgb(8, 8, 8)',
                          borderBottomLeftRadius: 6,
                          borderBottomRightRadius: 6,
                          borderTopLeftRadius: 6,
                          borderTopRightRadius: 6,
                          boxShadow:
                            '0px 0.7961918735236395px 2.3885756205709185px -0.625px rgba(0, 0, 0, 0.05), 0px 2.414506143104518px 7.2435184293135535px -1.25px rgba(0, 0, 0, 0.05), 0px 6.382653521484461px 19.147960564453385px -1.875px rgba(0, 0, 0, 0.05), 0px 20px 60px -2.5px rgba(0, 0, 0, 0.05)',
                        },
                        children: [
                          /* @__PURE__ */ _jsx5(RichText4, {
                            __fromCanvasComponent: true,
                            children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                              children: /* @__PURE__ */ _jsx5(motion4.p, {
                                style: {
                                  '--font-selector': 'R0Y7SW50ZXItNjAw',
                                  '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                  '--framer-font-size': '24px',
                                  '--framer-font-weight': '600',
                                  '--framer-line-height': '1.5em',
                                  '--framer-text-alignment': 'left',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                                },
                                children: 'Pro',
                              },),
                            },),
                            className: 'framer-x9ykqk',
                            fonts: ['GF;Inter-600',],
                            layoutDependency,
                            layoutId: 'ElU64kMGQ',
                            style: {
                              '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                              '--framer-link-text-color': 'rgb(0, 153, 255)',
                              '--framer-link-text-decoration': 'underline',
                              '--framer-paragraph-spacing': '0px',
                            },
                            verticalAlignment: 'top',
                            withExternalLayout: true,
                          },),
                          /* @__PURE__ */ _jsxs4(motion4.div, {
                            className: 'framer-17265uq',
                            layoutDependency,
                            layoutId: 't0grToHH0',
                            children: [
                              /* @__PURE__ */ _jsx5(RichText4, {
                                __fromCanvasComponent: true,
                                children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                  children: /* @__PURE__ */ _jsx5(motion4.p, {
                                    style: {
                                      '--font-selector': 'R0Y7SW50ZXItNzAw',
                                      '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                      '--framer-font-size': '40px',
                                      '--framer-font-weight': '700',
                                      '--framer-letter-spacing': '-3px',
                                      '--framer-line-height': '1em',
                                      '--framer-text-alignment': 'left',
                                      '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                                    },
                                    children: '$20',
                                  },),
                                },),
                                className: 'framer-17ziua4',
                                fonts: ['GF;Inter-700',],
                                layoutDependency,
                                layoutId: 'B414hrCcX',
                                style: {
                                  '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                                  '--framer-link-text-color': 'rgb(0, 153, 255)',
                                  '--framer-link-text-decoration': 'underline',
                                  '--framer-paragraph-spacing': '0px',
                                },
                                verticalAlignment: 'top',
                                withExternalLayout: true,
                              },),
                              /* @__PURE__ */ _jsx5(RichText4, {
                                __fromCanvasComponent: true,
                                children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                  children: /* @__PURE__ */ _jsx5(motion4.p, {
                                    style: {
                                      '--font-selector': 'R0Y7SW50ZXItNjAw',
                                      '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                      '--framer-font-size': '14px',
                                      '--framer-font-weight': '600',
                                      '--framer-line-height': '1.5em',
                                      '--framer-text-alignment': 'left',
                                      '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                    },
                                    children: 'month',
                                  },),
                                },),
                                className: 'framer-ff6fz9',
                                fonts: ['GF;Inter-600',],
                                layoutDependency,
                                layoutId: 'XEUgY8EHd',
                                style: {
                                  '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                  '--framer-link-text-color': 'rgb(0, 153, 255)',
                                  '--framer-link-text-decoration': 'underline',
                                  '--framer-paragraph-spacing': '0px',
                                },
                                verticalAlignment: 'top',
                                withExternalLayout: true,
                              },),
                            ],
                          },),
                          /* @__PURE__ */ _jsx5(RichText4, {
                            __fromCanvasComponent: true,
                            children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                              children: /* @__PURE__ */ _jsx5(motion4.p, {
                                style: {
                                  '--font-selector': 'R0Y7SW50ZXItNjAw',
                                  '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                  '--framer-font-size': '14px',
                                  '--framer-font-weight': '600',
                                  '--framer-line-height': '1.5em',
                                  '--framer-text-alignment': 'left',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                },
                                children: '+Resource Usage',
                              },),
                            },),
                            className: 'framer-1vee65n',
                            fonts: ['GF;Inter-600',],
                            layoutDependency,
                            layoutId: 'NZNual0Z0',
                            style: {
                              '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                              '--framer-link-text-color': 'rgb(0, 153, 255)',
                              '--framer-link-text-decoration': 'underline',
                              '--framer-paragraph-spacing': '0px',
                            },
                            verticalAlignment: 'top',
                            withExternalLayout: true,
                          },),
                          /* @__PURE__ */ _jsxs4(motion4.div, {
                            className: 'framer-18995ll',
                            layoutDependency,
                            layoutId: 'dhBhtwMps',
                            children: [
                              /* @__PURE__ */ _jsxs4(motion4.div, {
                                className: 'framer-zd13hn',
                                'data-framer-name': 'Row',
                                layoutDependency,
                                layoutId: 'v8EWzrVcC',
                                children: [
                                  /* @__PURE__ */ _jsx5(SVG3, {
                                    className: 'framer-1d9mh93',
                                    'data-framer-name': 'Check',
                                    layout: 'position',
                                    layoutDependency,
                                    layoutId: 'euShYHVLr',
                                    opacity: 1,
                                    radius: 0,
                                    style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                    svg:
                                      '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                    svgContentId: 12326142209,
                                    withExternalLayout: true,
                                  },),
                                  /* @__PURE__ */ _jsx5(RichText4, {
                                    __fromCanvasComponent: true,
                                    children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                      children: /* @__PURE__ */ _jsx5(motion4.p, {
                                        style: {
                                          '--font-selector': 'R0Y7SW50ZXItNjAw',
                                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                          '--framer-font-size': '12px',
                                          '--framer-font-weight': '600',
                                          '--framer-line-height': '1.5em',
                                          '--framer-text-alignment': 'left',
                                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                        },
                                        children: '2 projects',
                                      },),
                                    },),
                                    className: 'framer-68lhlv',
                                    fonts: ['GF;Inter-600',],
                                    layoutDependency,
                                    layoutId: 'yfHj2PPeh',
                                    style: {
                                      '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                      '--framer-link-text-color': 'rgb(0, 153, 255)',
                                      '--framer-link-text-decoration': 'underline',
                                      '--framer-paragraph-spacing': '0px',
                                    },
                                    verticalAlignment: 'top',
                                    withExternalLayout: true,
                                  },),
                                ],
                              },),
                              /* @__PURE__ */ _jsxs4(motion4.div, {
                                className: 'framer-md98au',
                                'data-framer-name': 'Row',
                                layoutDependency,
                                layoutId: 'NlzRMwKez',
                                children: [
                                  /* @__PURE__ */ _jsx5(SVG3, {
                                    className: 'framer-1qx1tug',
                                    'data-framer-name': 'Check',
                                    layout: 'position',
                                    layoutDependency,
                                    layoutId: 'hJppmMMsh',
                                    opacity: 1,
                                    radius: 0,
                                    style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                    svg:
                                      '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                    svgContentId: 12326142209,
                                    withExternalLayout: true,
                                  },),
                                  /* @__PURE__ */ _jsx5(RichText4, {
                                    __fromCanvasComponent: true,
                                    children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                      children: /* @__PURE__ */ _jsx5(motion4.p, {
                                        style: {
                                          '--font-selector': 'R0Y7SW50ZXItNjAw',
                                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                          '--framer-font-size': '12px',
                                          '--framer-font-weight': '600',
                                          '--framer-line-height': '1.5em',
                                          '--framer-text-alignment': 'left',
                                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                        },
                                        children: 'Analytics',
                                      },),
                                    },),
                                    className: 'framer-14js4mt',
                                    fonts: ['GF;Inter-600',],
                                    layoutDependency,
                                    layoutId: 'WYPO1V5ss',
                                    style: {
                                      '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                      '--framer-link-text-color': 'rgb(0, 153, 255)',
                                      '--framer-link-text-decoration': 'underline',
                                      '--framer-paragraph-spacing': '0px',
                                    },
                                    verticalAlignment: 'top',
                                    withExternalLayout: true,
                                  },),
                                ],
                              },),
                              /* @__PURE__ */ _jsxs4(motion4.div, {
                                className: 'framer-1uujxml',
                                'data-framer-name': 'Row',
                                layoutDependency,
                                layoutId: 'HPfIEQTrk',
                                children: [
                                  /* @__PURE__ */ _jsx5(SVG3, {
                                    className: 'framer-1bp6goi',
                                    'data-framer-name': 'Check',
                                    layout: 'position',
                                    layoutDependency,
                                    layoutId: 'ZqKl7gO1r',
                                    opacity: 1,
                                    radius: 0,
                                    style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                    svg:
                                      '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                    svgContentId: 12326142209,
                                    withExternalLayout: true,
                                  },),
                                  /* @__PURE__ */ _jsx5(RichText4, {
                                    __fromCanvasComponent: true,
                                    children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                      children: /* @__PURE__ */ _jsx5(motion4.p, {
                                        style: {
                                          '--font-selector': 'R0Y7SW50ZXItNjAw',
                                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                          '--framer-font-size': '12px',
                                          '--framer-font-weight': '600',
                                          '--framer-line-height': '1.5em',
                                          '--framer-text-alignment': 'left',
                                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                        },
                                        children: 'Email Support',
                                      },),
                                    },),
                                    className: 'framer-5xzkik',
                                    fonts: ['GF;Inter-600',],
                                    layoutDependency,
                                    layoutId: 'B6_JnZ4qB',
                                    style: {
                                      '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                      '--framer-link-text-color': 'rgb(0, 153, 255)',
                                      '--framer-link-text-decoration': 'underline',
                                      '--framer-paragraph-spacing': '0px',
                                    },
                                    verticalAlignment: 'top',
                                    withExternalLayout: true,
                                  },),
                                ],
                              },),
                              /* @__PURE__ */ _jsxs4(motion4.div, {
                                className: 'framer-9kzb8d',
                                'data-framer-name': 'Row',
                                layoutDependency,
                                layoutId: 'mdullaFzy',
                                children: [
                                  /* @__PURE__ */ _jsx5(SVG3, {
                                    className: 'framer-iixjq0',
                                    'data-framer-name': 'Check',
                                    layout: 'position',
                                    layoutDependency,
                                    layoutId: 'uXxxYvF4b',
                                    opacity: 1,
                                    radius: 0,
                                    style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                    svg:
                                      '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                    svgContentId: 12326142209,
                                    withExternalLayout: true,
                                  },),
                                  /* @__PURE__ */ _jsx5(RichText4, {
                                    __fromCanvasComponent: true,
                                    children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                      children: /* @__PURE__ */ _jsx5(motion4.p, {
                                        style: {
                                          '--font-selector': 'R0Y7SW50ZXItNjAw',
                                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                          '--framer-font-size': '12px',
                                          '--framer-font-weight': '600',
                                          '--framer-line-height': '1.5em',
                                          '--framer-text-alignment': 'left',
                                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                        },
                                        children: 'Share Features',
                                      },),
                                    },),
                                    className: 'framer-zki8t0',
                                    fonts: ['GF;Inter-600',],
                                    layoutDependency,
                                    layoutId: 'Y5BJDun6j',
                                    style: {
                                      '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                      '--framer-link-text-color': 'rgb(0, 153, 255)',
                                      '--framer-link-text-decoration': 'underline',
                                      '--framer-paragraph-spacing': '0px',
                                    },
                                    verticalAlignment: 'top',
                                    withExternalLayout: true,
                                  },),
                                ],
                              },),
                            ],
                          },),
                          /* @__PURE__ */ _jsx5(motion4.div, {
                            className: 'framer-175vey9',
                            'data-framer-name': 'Button',
                            layoutDependency,
                            layoutId: 'uTFUdQq1m',
                            style: {
                              backgroundColor: 'rgb(255, 79, 0)',
                              borderBottomLeftRadius: 4,
                              borderBottomRightRadius: 4,
                              borderTopLeftRadius: 4,
                              borderTopRightRadius: 4,
                              boxShadow:
                                '0px 0.7065919983928324px 0.7065919983928324px -0.625px rgba(0, 0, 0, 0.14764), 0px 1.8065619053231785px 1.8065619053231785px -1.25px rgba(0, 0, 0, 0.14398), 0px 3.6217592146567767px 3.6217592146567767px -1.875px rgba(0, 0, 0, 0.13793), 0px 6.8655999097303715px 6.8655999097303715px -2.5px rgba(0, 0, 0, 0.12711), 0px 13.646761411524492px 13.646761411524492px -3.125px rgba(0, 0, 0, 0.10451), 0px 30px 30px -3.75px rgba(0, 0, 0, 0.05)',
                            },
                            children: /* @__PURE__ */ _jsx5(RichText4, {
                              __fromCanvasComponent: true,
                              children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                children: /* @__PURE__ */ _jsx5(motion4.p, {
                                  style: {
                                    '--font-selector': 'R0Y7SW50ZXItNjAw',
                                    '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                    '--framer-font-size': '14px',
                                    '--framer-font-weight': '600',
                                    '--framer-letter-spacing': '0px',
                                    '--framer-text-alignment': 'center',
                                    '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                                  },
                                  children: 'Get Started',
                                },),
                              },),
                              className: 'framer-1hslzhu',
                              fonts: ['GF;Inter-600',],
                              layoutDependency,
                              layoutId: 'LzqE1AyV1',
                              style: {
                                '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                                '--framer-link-text-color': 'rgb(0, 153, 255)',
                                '--framer-link-text-decoration': 'underline',
                                '--framer-paragraph-spacing': '0px',
                              },
                              verticalAlignment: 'top',
                              withExternalLayout: true,
                            },),
                          },),
                        ],
                      },),
                      /* @__PURE__ */ _jsxs4(motion4.div, {
                        className: 'framer-19e67rc',
                        'data-border': true,
                        'data-framer-name': 'Card',
                        layoutDependency,
                        layoutId: 'tw7uFq0IH',
                        style: {
                          '--border-bottom-width': '1px',
                          '--border-color': 'rgb(77, 77, 77)',
                          '--border-left-width': '1px',
                          '--border-right-width': '1px',
                          '--border-style': 'solid',
                          '--border-top-width': '1px',
                          backgroundColor: 'rgb(8, 8, 8)',
                          borderBottomLeftRadius: 6,
                          borderBottomRightRadius: 6,
                          borderTopLeftRadius: 6,
                          borderTopRightRadius: 6,
                          boxShadow:
                            '0px 0.7961918735236395px 2.3885756205709185px -0.625px rgba(0, 0, 0, 0.05), 0px 2.414506143104518px 7.2435184293135535px -1.25px rgba(0, 0, 0, 0.05), 0px 6.382653521484461px 19.147960564453385px -1.875px rgba(0, 0, 0, 0.05), 0px 20px 60px -2.5px rgba(0, 0, 0, 0.05)',
                        },
                        children: [
                          /* @__PURE__ */ _jsx5(RichText4, {
                            __fromCanvasComponent: true,
                            children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                              children: /* @__PURE__ */ _jsx5(motion4.p, {
                                style: {
                                  '--font-selector': 'R0Y7SW50ZXItNjAw',
                                  '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                  '--framer-font-size': '24px',
                                  '--framer-font-weight': '600',
                                  '--framer-line-height': '1.5em',
                                  '--framer-text-alignment': 'left',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                                },
                                children: 'Team',
                              },),
                            },),
                            className: 'framer-arkonl',
                            fonts: ['GF;Inter-600',],
                            layoutDependency,
                            layoutId: 'QxwRh6nEr',
                            style: {
                              '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                              '--framer-link-text-color': 'rgb(0, 153, 255)',
                              '--framer-link-text-decoration': 'underline',
                              '--framer-paragraph-spacing': '0px',
                            },
                            verticalAlignment: 'top',
                            withExternalLayout: true,
                          },),
                          /* @__PURE__ */ _jsxs4(motion4.div, {
                            className: 'framer-100vaex',
                            layoutDependency,
                            layoutId: 'wo_U2FqKj',
                            children: [
                              /* @__PURE__ */ _jsx5(RichText4, {
                                __fromCanvasComponent: true,
                                children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                  children: /* @__PURE__ */ _jsx5(motion4.p, {
                                    style: {
                                      '--font-selector': 'R0Y7SW50ZXItNzAw',
                                      '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                      '--framer-font-size': '40px',
                                      '--framer-font-weight': '700',
                                      '--framer-letter-spacing': '-3px',
                                      '--framer-line-height': '1em',
                                      '--framer-text-alignment': 'left',
                                      '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                                    },
                                    children: '$200',
                                  },),
                                },),
                                className: 'framer-qn0o5u',
                                fonts: ['GF;Inter-700',],
                                layoutDependency,
                                layoutId: 'CpgkCE8ko',
                                style: {
                                  '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                                  '--framer-link-text-color': 'rgb(0, 153, 255)',
                                  '--framer-link-text-decoration': 'underline',
                                  '--framer-paragraph-spacing': '0px',
                                },
                                verticalAlignment: 'top',
                                withExternalLayout: true,
                              },),
                              /* @__PURE__ */ _jsx5(RichText4, {
                                __fromCanvasComponent: true,
                                children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                  children: /* @__PURE__ */ _jsx5(motion4.p, {
                                    style: {
                                      '--font-selector': 'R0Y7SW50ZXItNjAw',
                                      '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                      '--framer-font-size': '14px',
                                      '--framer-font-weight': '600',
                                      '--framer-line-height': '1.5em',
                                      '--framer-text-alignment': 'left',
                                      '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                    },
                                    children: 'month',
                                  },),
                                },),
                                className: 'framer-tzreq4',
                                fonts: ['GF;Inter-600',],
                                layoutDependency,
                                layoutId: 'cnZ3R0hCD',
                                style: {
                                  '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                  '--framer-link-text-color': 'rgb(0, 153, 255)',
                                  '--framer-link-text-decoration': 'underline',
                                  '--framer-paragraph-spacing': '0px',
                                },
                                verticalAlignment: 'top',
                                withExternalLayout: true,
                              },),
                            ],
                          },),
                          /* @__PURE__ */ _jsx5(RichText4, {
                            __fromCanvasComponent: true,
                            children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                              children: /* @__PURE__ */ _jsx5(motion4.p, {
                                style: {
                                  '--font-selector': 'R0Y7SW50ZXItNjAw',
                                  '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                  '--framer-font-size': '12px',
                                  '--framer-font-weight': '600',
                                  '--framer-line-height': '1.5em',
                                  '--framer-text-alignment': 'left',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                },
                                children: '+Resource Usage',
                              },),
                            },),
                            className: 'framer-1kp0bhg',
                            fonts: ['GF;Inter-600',],
                            layoutDependency,
                            layoutId: 'zuJxX4v2r',
                            style: {
                              '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                              '--framer-link-text-color': 'rgb(0, 153, 255)',
                              '--framer-link-text-decoration': 'underline',
                              '--framer-paragraph-spacing': '0px',
                            },
                            verticalAlignment: 'top',
                            withExternalLayout: true,
                          },),
                          /* @__PURE__ */ _jsxs4(motion4.div, {
                            className: 'framer-1svare3',
                            layoutDependency,
                            layoutId: 'oOG51SOCM',
                            children: [
                              /* @__PURE__ */ _jsxs4(motion4.div, {
                                className: 'framer-1hazcdv',
                                'data-framer-name': 'Row',
                                layoutDependency,
                                layoutId: 'I6JFQsYwy',
                                children: [
                                  /* @__PURE__ */ _jsx5(SVG3, {
                                    className: 'framer-xoy07e',
                                    'data-framer-name': 'Check',
                                    layout: 'position',
                                    layoutDependency,
                                    layoutId: 'RhEFVCjFK',
                                    opacity: 1,
                                    radius: 0,
                                    style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                    svg:
                                      '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                    svgContentId: 12129822493,
                                    withExternalLayout: true,
                                  },),
                                  /* @__PURE__ */ _jsx5(RichText4, {
                                    __fromCanvasComponent: true,
                                    children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                      children: /* @__PURE__ */ _jsx5(motion4.p, {
                                        style: {
                                          '--font-selector': 'R0Y7SW50ZXItNjAw',
                                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                          '--framer-font-size': '12px',
                                          '--framer-font-weight': '600',
                                          '--framer-line-height': '1.5em',
                                          '--framer-text-alignment': 'left',
                                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                        },
                                        children: 'AWS + G Cloud + Azure',
                                      },),
                                    },),
                                    className: 'framer-1ptd02p',
                                    fonts: ['GF;Inter-600',],
                                    layoutDependency,
                                    layoutId: 'dh6J32zWb',
                                    style: {
                                      '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                      '--framer-link-text-color': 'rgb(0, 153, 255)',
                                      '--framer-link-text-decoration': 'underline',
                                      '--framer-paragraph-spacing': '0px',
                                    },
                                    verticalAlignment: 'top',
                                    withExternalLayout: true,
                                  },),
                                ],
                              },),
                              /* @__PURE__ */ _jsxs4(motion4.div, {
                                className: 'framer-e5fm8s',
                                'data-framer-name': 'Row',
                                layoutDependency,
                                layoutId: 'qWCmZMkor',
                                children: [
                                  /* @__PURE__ */ _jsx5(SVG3, {
                                    className: 'framer-dt7grq',
                                    'data-framer-name': 'Check',
                                    layout: 'position',
                                    layoutDependency,
                                    layoutId: 'J7QcsaTMG',
                                    opacity: 1,
                                    radius: 0,
                                    style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                    svg:
                                      '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                    svgContentId: 12129822493,
                                    withExternalLayout: true,
                                  },),
                                  /* @__PURE__ */ _jsx5(RichText4, {
                                    __fromCanvasComponent: true,
                                    children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                      children: /* @__PURE__ */ _jsx5(motion4.p, {
                                        style: {
                                          '--font-selector': 'R0Y7SW50ZXItNjAw',
                                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                          '--framer-font-size': '12px',
                                          '--framer-font-weight': '600',
                                          '--framer-line-height': '1.5em',
                                          '--framer-text-alignment': 'left',
                                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                        },
                                        children: 'Analytics',
                                      },),
                                    },),
                                    className: 'framer-1cglloq',
                                    fonts: ['GF;Inter-600',],
                                    layoutDependency,
                                    layoutId: 'HHR8QeHRN',
                                    style: {
                                      '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                      '--framer-link-text-color': 'rgb(0, 153, 255)',
                                      '--framer-link-text-decoration': 'underline',
                                      '--framer-paragraph-spacing': '0px',
                                    },
                                    verticalAlignment: 'top',
                                    withExternalLayout: true,
                                  },),
                                ],
                              },),
                              /* @__PURE__ */ _jsxs4(motion4.div, {
                                className: 'framer-r2m7ui',
                                'data-framer-name': 'Row',
                                layoutDependency,
                                layoutId: 'SjC4PxH3o',
                                children: [
                                  /* @__PURE__ */ _jsx5(SVG3, {
                                    className: 'framer-ad17s4',
                                    'data-framer-name': 'Check',
                                    layout: 'position',
                                    layoutDependency,
                                    layoutId: 'SX7RkjRKm',
                                    opacity: 1,
                                    radius: 0,
                                    style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                    svg:
                                      '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                    svgContentId: 12129822493,
                                    withExternalLayout: true,
                                  },),
                                  /* @__PURE__ */ _jsx5(RichText4, {
                                    __fromCanvasComponent: true,
                                    children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                      children: /* @__PURE__ */ _jsx5(motion4.p, {
                                        style: {
                                          '--font-selector': 'R0Y7SW50ZXItNjAw',
                                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                          '--framer-font-size': '12px',
                                          '--framer-font-weight': '600',
                                          '--framer-line-height': '1.5em',
                                          '--framer-text-alignment': 'left',
                                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                        },
                                        children: 'Advance Support',
                                      },),
                                    },),
                                    className: 'framer-cecdzl',
                                    fonts: ['GF;Inter-600',],
                                    layoutDependency,
                                    layoutId: 'yydCn8IOy',
                                    style: {
                                      '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                      '--framer-link-text-color': 'rgb(0, 153, 255)',
                                      '--framer-link-text-decoration': 'underline',
                                      '--framer-paragraph-spacing': '0px',
                                    },
                                    verticalAlignment: 'top',
                                    withExternalLayout: true,
                                  },),
                                ],
                              },),
                              /* @__PURE__ */ _jsxs4(motion4.div, {
                                className: 'framer-mpf025',
                                'data-framer-name': 'Row',
                                layoutDependency,
                                layoutId: 'qGtpBYdz9',
                                children: [
                                  /* @__PURE__ */ _jsx5(SVG3, {
                                    className: 'framer-14xvxp9',
                                    'data-framer-name': 'Check',
                                    layout: 'position',
                                    layoutDependency,
                                    layoutId: 'iI7YDd0l4',
                                    opacity: 1,
                                    radius: 0,
                                    style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                    svg:
                                      '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                    svgContentId: 12129822493,
                                    withExternalLayout: true,
                                  },),
                                  /* @__PURE__ */ _jsx5(RichText4, {
                                    __fromCanvasComponent: true,
                                    children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                      children: /* @__PURE__ */ _jsx5(motion4.p, {
                                        style: {
                                          '--font-selector': 'R0Y7SW50ZXItNjAw',
                                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                          '--framer-font-size': '12px',
                                          '--framer-font-weight': '600',
                                          '--framer-line-height': '1.5em',
                                          '--framer-text-alignment': 'left',
                                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                        },
                                        children: 'Share Features',
                                      },),
                                    },),
                                    className: 'framer-q8icxh',
                                    fonts: ['GF;Inter-600',],
                                    layoutDependency,
                                    layoutId: 'J2A3LeSTs',
                                    style: {
                                      '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                      '--framer-link-text-color': 'rgb(0, 153, 255)',
                                      '--framer-link-text-decoration': 'underline',
                                      '--framer-paragraph-spacing': '0px',
                                    },
                                    verticalAlignment: 'top',
                                    withExternalLayout: true,
                                  },),
                                ],
                              },),
                            ],
                          },),
                          /* @__PURE__ */ _jsx5(motion4.div, {
                            className: 'framer-138tvcp',
                            'data-framer-name': 'Button',
                            layoutDependency,
                            layoutId: 'y7h5L8ZuE',
                            style: {
                              backgroundColor: 'rgb(255, 79, 0)',
                              borderBottomLeftRadius: 4,
                              borderBottomRightRadius: 4,
                              borderTopLeftRadius: 4,
                              borderTopRightRadius: 4,
                              boxShadow:
                                '0px 0.7065919983928324px 0.7065919983928324px -0.625px rgba(0, 0, 0, 0.14764), 0px 1.8065619053231785px 1.8065619053231785px -1.25px rgba(0, 0, 0, 0.14398), 0px 3.6217592146567767px 3.6217592146567767px -1.875px rgba(0, 0, 0, 0.13793), 0px 6.8655999097303715px 6.8655999097303715px -2.5px rgba(0, 0, 0, 0.12711), 0px 13.646761411524492px 13.646761411524492px -3.125px rgba(0, 0, 0, 0.10451), 0px 30px 30px -3.75px rgba(0, 0, 0, 0.05)',
                            },
                            children: /* @__PURE__ */ _jsx5(RichText4, {
                              __fromCanvasComponent: true,
                              children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                children: /* @__PURE__ */ _jsx5(motion4.p, {
                                  style: {
                                    '--font-selector': 'R0Y7SW50ZXItNjAw',
                                    '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                    '--framer-font-size': '14px',
                                    '--framer-font-weight': '600',
                                    '--framer-letter-spacing': '0px',
                                    '--framer-text-alignment': 'center',
                                    '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                                  },
                                  children: 'Get Started',
                                },),
                              },),
                              className: 'framer-fjanm4',
                              fonts: ['GF;Inter-600',],
                              layoutDependency,
                              layoutId: 'ot9moWQYB',
                              style: {
                                '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                                '--framer-link-text-color': 'rgb(0, 153, 255)',
                                '--framer-link-text-decoration': 'underline',
                                '--framer-paragraph-spacing': '0px',
                              },
                              verticalAlignment: 'top',
                              withExternalLayout: true,
                            },),
                          },),
                        ],
                      },),
                      /* @__PURE__ */ _jsxs4(motion4.div, {
                        className: 'framer-mrnxno',
                        'data-border': true,
                        'data-framer-name': 'Card',
                        layoutDependency,
                        layoutId: 'SYYaH2fD6',
                        style: {
                          '--border-bottom-width': '1px',
                          '--border-color': 'rgb(77, 77, 77)',
                          '--border-left-width': '1px',
                          '--border-right-width': '1px',
                          '--border-style': 'solid',
                          '--border-top-width': '1px',
                          backgroundColor: 'rgb(8, 8, 8)',
                          borderBottomLeftRadius: 6,
                          borderBottomRightRadius: 6,
                          borderTopLeftRadius: 6,
                          borderTopRightRadius: 6,
                          boxShadow:
                            '0px 0.7961918735236395px 2.3885756205709185px -0.625px rgba(0, 0, 0, 0.05), 0px 2.414506143104518px 7.2435184293135535px -1.25px rgba(0, 0, 0, 0.05), 0px 6.382653521484461px 19.147960564453385px -1.875px rgba(0, 0, 0, 0.05), 0px 20px 60px -2.5px rgba(0, 0, 0, 0.05)',
                        },
                        children: [
                          /* @__PURE__ */ _jsx5(RichText4, {
                            __fromCanvasComponent: true,
                            children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                              children: /* @__PURE__ */ _jsx5(motion4.p, {
                                style: {
                                  '--font-selector': 'R0Y7SW50ZXItNjAw',
                                  '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                  '--framer-font-size': '24px',
                                  '--framer-font-weight': '600',
                                  '--framer-line-height': '1.5em',
                                  '--framer-text-alignment': 'left',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                                },
                                children: 'Enterprise',
                              },),
                            },),
                            className: 'framer-f1ay96',
                            fonts: ['GF;Inter-600',],
                            layoutDependency,
                            layoutId: 'gWuQfO2tI',
                            style: {
                              '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                              '--framer-link-text-color': 'rgb(0, 153, 255)',
                              '--framer-link-text-decoration': 'underline',
                              '--framer-paragraph-spacing': '0px',
                            },
                            verticalAlignment: 'top',
                            withExternalLayout: true,
                          },),
                          /* @__PURE__ */ _jsx5(motion4.div, {
                            className: 'framer-1s8u59l',
                            layoutDependency,
                            layoutId: 'r71Jlst7v',
                            children: /* @__PURE__ */ _jsx5(RichText4, {
                              __fromCanvasComponent: true,
                              children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                children: /* @__PURE__ */ _jsx5(motion4.p, {
                                  style: {
                                    '--font-selector': 'R0Y7SW50ZXItNzAw',
                                    '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                    '--framer-font-size': '40px',
                                    '--framer-font-weight': '700',
                                    '--framer-letter-spacing': '-3px',
                                    '--framer-line-height': '1em',
                                    '--framer-text-alignment': 'left',
                                    '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                                  },
                                  children: 'Custom',
                                },),
                              },),
                              className: 'framer-94oxdl',
                              fonts: ['GF;Inter-700',],
                              layoutDependency,
                              layoutId: 'Ud55CmkhA',
                              style: {
                                '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                                '--framer-link-text-color': 'rgb(0, 153, 255)',
                                '--framer-link-text-decoration': 'underline',
                                '--framer-paragraph-spacing': '0px',
                              },
                              verticalAlignment: 'top',
                              withExternalLayout: true,
                            },),
                          },),
                          /* @__PURE__ */ _jsx5(RichText4, {
                            __fromCanvasComponent: true,
                            children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                              children: /* @__PURE__ */ _jsx5(motion4.p, {
                                style: {
                                  '--font-selector': 'R0Y7SW50ZXItNjAw',
                                  '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                  '--framer-font-size': '14px',
                                  '--framer-font-weight': '600',
                                  '--framer-line-height': '1.5em',
                                  '--framer-text-alignment': 'left',
                                  '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                },
                                children: ' ',
                              },),
                            },),
                            className: 'framer-ur8tni',
                            fonts: ['GF;Inter-600',],
                            layoutDependency,
                            layoutId: 'gEWeRBTnM',
                            style: {
                              '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                              '--framer-link-text-color': 'rgb(0, 153, 255)',
                              '--framer-link-text-decoration': 'underline',
                              '--framer-paragraph-spacing': '0px',
                            },
                            verticalAlignment: 'top',
                            withExternalLayout: true,
                          },),
                          /* @__PURE__ */ _jsxs4(motion4.div, {
                            className: 'framer-19vyt0w',
                            layoutDependency,
                            layoutId: 'MdbvNPWt3',
                            children: [
                              /* @__PURE__ */ _jsxs4(motion4.div, {
                                className: 'framer-mxjfz6',
                                'data-framer-name': 'Row',
                                layoutDependency,
                                layoutId: 'YZ8rapluk',
                                children: [
                                  /* @__PURE__ */ _jsx5(SVG3, {
                                    className: 'framer-1xguxuj',
                                    'data-framer-name': 'Check',
                                    layout: 'position',
                                    layoutDependency,
                                    layoutId: 'HYaNDwULd',
                                    opacity: 1,
                                    radius: 0,
                                    style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                    svg:
                                      '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                    svgContentId: 12129822493,
                                    withExternalLayout: true,
                                  },),
                                  /* @__PURE__ */ _jsx5(RichText4, {
                                    __fromCanvasComponent: true,
                                    children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                      children: /* @__PURE__ */ _jsx5(motion4.p, {
                                        style: {
                                          '--font-selector': 'R0Y7SW50ZXItNjAw',
                                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                          '--framer-font-size': '12px',
                                          '--framer-font-weight': '600',
                                          '--framer-line-height': '1.5em',
                                          '--framer-text-alignment': 'left',
                                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                        },
                                        children: 'Unlimited Projects',
                                      },),
                                    },),
                                    className: 'framer-505x1r',
                                    fonts: ['GF;Inter-600',],
                                    layoutDependency,
                                    layoutId: 'ltpa00AF0',
                                    style: {
                                      '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                      '--framer-link-text-color': 'rgb(0, 153, 255)',
                                      '--framer-link-text-decoration': 'underline',
                                      '--framer-paragraph-spacing': '0px',
                                    },
                                    verticalAlignment: 'top',
                                    withExternalLayout: true,
                                  },),
                                ],
                              },),
                              /* @__PURE__ */ _jsxs4(motion4.div, {
                                className: 'framer-yafzz',
                                'data-framer-name': 'Row',
                                layoutDependency,
                                layoutId: 'dvaZQ1YTR',
                                children: [
                                  /* @__PURE__ */ _jsx5(SVG3, {
                                    className: 'framer-17ro9bq',
                                    'data-framer-name': 'Check',
                                    layout: 'position',
                                    layoutDependency,
                                    layoutId: 'mxP03tiJE',
                                    opacity: 1,
                                    radius: 0,
                                    style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                    svg:
                                      '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                    svgContentId: 12129822493,
                                    withExternalLayout: true,
                                  },),
                                  /* @__PURE__ */ _jsx5(RichText4, {
                                    __fromCanvasComponent: true,
                                    children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                      children: /* @__PURE__ */ _jsx5(motion4.p, {
                                        style: {
                                          '--font-selector': 'R0Y7SW50ZXItNjAw',
                                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                          '--framer-font-size': '12px',
                                          '--framer-font-weight': '600',
                                          '--framer-line-height': '1.5em',
                                          '--framer-text-alignment': 'left',
                                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                        },
                                        children: 'Priority Support',
                                      },),
                                    },),
                                    className: 'framer-1wms3xz',
                                    fonts: ['GF;Inter-600',],
                                    layoutDependency,
                                    layoutId: 'Gmewc2lo5',
                                    style: {
                                      '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                      '--framer-link-text-color': 'rgb(0, 153, 255)',
                                      '--framer-link-text-decoration': 'underline',
                                      '--framer-paragraph-spacing': '0px',
                                    },
                                    verticalAlignment: 'top',
                                    withExternalLayout: true,
                                  },),
                                ],
                              },),
                              /* @__PURE__ */ _jsxs4(motion4.div, {
                                className: 'framer-w3cual',
                                'data-framer-name': 'Row',
                                layoutDependency,
                                layoutId: 'IAlPhqfbr',
                                children: [
                                  /* @__PURE__ */ _jsx5(SVG3, {
                                    className: 'framer-jq93oi',
                                    'data-framer-name': 'Check',
                                    layout: 'position',
                                    layoutDependency,
                                    layoutId: 'ZLOo3fvVH',
                                    opacity: 1,
                                    radius: 0,
                                    style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                    svg:
                                      '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                    svgContentId: 12129822493,
                                    withExternalLayout: true,
                                  },),
                                  /* @__PURE__ */ _jsx5(RichText4, {
                                    __fromCanvasComponent: true,
                                    children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                      children: /* @__PURE__ */ _jsx5(motion4.p, {
                                        style: {
                                          '--font-selector': 'R0Y7SW50ZXItNjAw',
                                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                          '--framer-font-size': '12px',
                                          '--framer-font-weight': '600',
                                          '--framer-line-height': '1.5em',
                                          '--framer-text-alignment': 'left',
                                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                        },
                                        children: '99.99% SLA',
                                      },),
                                    },),
                                    className: 'framer-1csuvb5',
                                    fonts: ['GF;Inter-600',],
                                    layoutDependency,
                                    layoutId: 'TCYw10NSo',
                                    style: {
                                      '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                      '--framer-link-text-color': 'rgb(0, 153, 255)',
                                      '--framer-link-text-decoration': 'underline',
                                      '--framer-paragraph-spacing': '0px',
                                    },
                                    verticalAlignment: 'top',
                                    withExternalLayout: true,
                                  },),
                                ],
                              },),
                              /* @__PURE__ */ _jsxs4(motion4.div, {
                                className: 'framer-15vzggn',
                                'data-framer-name': 'Row',
                                layoutDependency,
                                layoutId: 'YSJk5J7sb',
                                children: [
                                  /* @__PURE__ */ _jsx5(SVG3, {
                                    className: 'framer-1tpv3ki',
                                    'data-framer-name': 'Check',
                                    layout: 'position',
                                    layoutDependency,
                                    layoutId: 'DJMbeE3wx',
                                    opacity: 1,
                                    radius: 0,
                                    style: { backgroundColor: 'rgba(0, 0, 0, 0)', },
                                    svg:
                                      '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16"><path d="M 8 1.5 C 4.41 1.5 1.5 4.41 1.5 8 C 1.5 11.59 4.41 14.5 8 14.5 C 11.59 14.5 14.5 11.59 14.5 8 C 14.493 4.413 11.587 1.507 8 1.5 Z M 11.094 6.863 L 7.431 10.363 C 7.337 10.452 7.211 10.501 7.081 10.5 C 6.953 10.502 6.829 10.452 6.737 10.363 L 4.906 8.613 C 4.768 8.492 4.707 8.305 4.747 8.126 C 4.787 7.947 4.923 7.804 5.099 7.754 C 5.276 7.705 5.466 7.756 5.594 7.888 L 7.081 9.306 L 10.406 6.138 C 10.608 5.961 10.913 5.976 11.098 6.17 C 11.282 6.365 11.28 6.67 11.094 6.863 Z" fill="rgb(255, 255, 255)"></path></svg>',
                                    svgContentId: 12129822493,
                                    withExternalLayout: true,
                                  },),
                                  /* @__PURE__ */ _jsx5(RichText4, {
                                    __fromCanvasComponent: true,
                                    children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                      children: /* @__PURE__ */ _jsx5(motion4.p, {
                                        style: {
                                          '--font-selector': 'R0Y7SW50ZXItNjAw',
                                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                          '--framer-font-size': '12px',
                                          '--framer-font-weight': '600',
                                          '--framer-line-height': '1.5em',
                                          '--framer-text-alignment': 'left',
                                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(102, 102, 102))',
                                        },
                                        children: 'OIDC SSO provider',
                                      },),
                                    },),
                                    className: 'framer-pom0w',
                                    fonts: ['GF;Inter-600',],
                                    layoutDependency,
                                    layoutId: 'AnwEUPvr2',
                                    style: {
                                      '--extracted-r6o4lv': 'rgb(102, 102, 102)',
                                      '--framer-link-text-color': 'rgb(0, 153, 255)',
                                      '--framer-link-text-decoration': 'underline',
                                      '--framer-paragraph-spacing': '0px',
                                    },
                                    verticalAlignment: 'top',
                                    withExternalLayout: true,
                                  },),
                                ],
                              },),
                            ],
                          },),
                          /* @__PURE__ */ _jsx5(motion4.div, {
                            className: 'framer-m7btq5',
                            'data-framer-name': 'Button',
                            layoutDependency,
                            layoutId: 'sBD75y8y1',
                            style: {
                              backgroundColor: 'rgb(255, 79, 0)',
                              borderBottomLeftRadius: 4,
                              borderBottomRightRadius: 4,
                              borderTopLeftRadius: 4,
                              borderTopRightRadius: 4,
                              boxShadow:
                                '0px 0.7065919983928324px 0.7065919983928324px -0.625px rgba(0, 0, 0, 0.14764), 0px 1.8065619053231785px 1.8065619053231785px -1.25px rgba(0, 0, 0, 0.14398), 0px 3.6217592146567767px 3.6217592146567767px -1.875px rgba(0, 0, 0, 0.13793), 0px 6.8655999097303715px 6.8655999097303715px -2.5px rgba(0, 0, 0, 0.12711), 0px 13.646761411524492px 13.646761411524492px -3.125px rgba(0, 0, 0, 0.10451), 0px 30px 30px -3.75px rgba(0, 0, 0, 0.05)',
                            },
                            children: /* @__PURE__ */ _jsx5(RichText4, {
                              __fromCanvasComponent: true,
                              children: /* @__PURE__ */ _jsx5(React5.Fragment, {
                                children: /* @__PURE__ */ _jsx5(motion4.p, {
                                  style: {
                                    '--font-selector': 'R0Y7SW50ZXItNjAw',
                                    '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                    '--framer-font-size': '14px',
                                    '--framer-font-weight': '600',
                                    '--framer-letter-spacing': '0px',
                                    '--framer-text-alignment': 'center',
                                    '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                                  },
                                  children: 'Contact Us',
                                },),
                              },),
                              className: 'framer-ke8bg0',
                              fonts: ['GF;Inter-600',],
                              layoutDependency,
                              layoutId: 'uNKV3wQbv',
                              style: {
                                '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                                '--framer-link-text-color': 'rgb(0, 153, 255)',
                                '--framer-link-text-decoration': 'underline',
                                '--framer-paragraph-spacing': '0px',
                              },
                              verticalAlignment: 'top',
                              withExternalLayout: true,
                            },),
                          },),
                        ],
                      },),
                    ],
                  },),
                },),
                /* @__PURE__ */ _jsxs4(motion4.div, {
                  className: 'framer-lc5uts',
                  layoutDependency,
                  layoutId: 'pbq2yeFp9',
                  children: [
                    /* @__PURE__ */ _jsx5(ComponentViewportProvider, {
                      height: 397,
                      ...addPropertyOverrides4({ OlQQ934Vt: { width: '240px', }, }, baseVariant, gestureVariant,),
                      children: /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-n2pkst-container',
                        layoutDependency,
                        layoutId: 'JbRe9QyVp-container',
                        children: /* @__PURE__ */ _jsx5(stdin_default3, {
                          height: '100%',
                          id: 'JbRe9QyVp',
                          layoutId: 'JbRe9QyVp',
                          style: { width: '100%', },
                          variant: 'Op3GPHO8w',
                          width: '100%',
                        },),
                      },),
                    },),
                    /* @__PURE__ */ _jsx5(ComponentViewportProvider, {
                      height: 397,
                      ...addPropertyOverrides4({ OlQQ934Vt: { width: '240px', }, }, baseVariant, gestureVariant,),
                      children: /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-ii5715-container',
                        layoutDependency,
                        layoutId: 'qkG46n4UG-container',
                        children: /* @__PURE__ */ _jsx5(stdin_default2, {
                          height: '100%',
                          id: 'qkG46n4UG',
                          layoutId: 'qkG46n4UG',
                          style: { width: '100%', },
                          variant: 'GnsxM81Tp',
                          width: '100%',
                        },),
                      },),
                    },),
                    /* @__PURE__ */ _jsx5(ComponentViewportProvider, {
                      height: 397,
                      ...addPropertyOverrides4({ OlQQ934Vt: { width: '240px', }, }, baseVariant, gestureVariant,),
                      children: /* @__PURE__ */ _jsx5(motion4.div, {
                        className: 'framer-hdkqhh-container',
                        layoutDependency,
                        layoutId: 'iz3Rj1gmD-container',
                        children: /* @__PURE__ */ _jsx5(stdin_default, {
                          height: '100%',
                          id: 'iz3Rj1gmD',
                          layoutId: 'iz3Rj1gmD',
                          style: { width: '100%', },
                          variant: 'cOu2SK2RI',
                          width: '100%',
                        },),
                      },),
                    },),
                  ],
                },),
              ],
            },),
          ],
        },),
      },),
    },),
  },);
},);
var css6 = [
  '@supports (aspect-ratio: 1) { body { --framer-aspect-ratio-supported: auto; } }',
  '.framer-i47EY.framer-194ejfq, .framer-i47EY .framer-194ejfq { display: block; }',
  '.framer-i47EY.framer-7simsb { align-content: center; align-items: center; display: flex; flex-direction: column; flex-wrap: nowrap; gap: 0px; height: 2526px; justify-content: flex-start; overflow: visible; padding: 144px 40px 144px 40px; position: relative; width: 1200px; }',
  '.framer-i47EY .framer-q4z8b, .framer-i47EY .framer-tlmrr2 { align-content: center; align-items: center; display: flex; flex: none; flex-direction: column; flex-wrap: nowrap; gap: 10px; height: min-content; justify-content: center; overflow: hidden; padding: 0px; position: relative; width: 100%; }',
  '.framer-i47EY .framer-yel58z, .framer-i47EY .framer-1hbajtu { flex: none; height: auto; overflow: visible; position: relative; white-space: pre-wrap; width: 100%; word-break: break-word; word-wrap: break-word; }',
  '.framer-i47EY .framer-58hm9z { flex: none; height: 96px; overflow: visible; position: relative; white-space: pre-wrap; width: 445px; word-break: break-word; word-wrap: break-word; }',
  '.framer-i47EY .framer-1voyydo { align-content: center; align-items: center; display: flex; flex: none; flex-direction: row; flex-wrap: nowrap; gap: 0px; height: min-content; justify-content: center; overflow: hidden; padding: 0px 0px 100px 0px; position: relative; width: 100%; }',
  '.framer-i47EY .framer-lbd5da, .framer-i47EY .framer-1i52f4v { align-content: center; align-items: center; display: flex; flex: 1 0 0px; flex-direction: row; flex-wrap: wrap; gap: 15px; height: min-content; justify-content: center; max-width: 1000px; overflow: visible; padding: 0px; position: relative; width: 1px; }',
  '.framer-i47EY .framer-1y83tpy, .framer-i47EY .framer-1p01z0t, .framer-i47EY .framer-lal9la, .framer-i47EY .framer-9qmgde, .framer-i47EY .framer-mimhod, .framer-i47EY .framer-h4jq97, .framer-i47EY .framer-19e67rc, .framer-i47EY .framer-mrnxno { align-content: flex-start; align-items: flex-start; display: flex; flex: none; flex-direction: column; flex-wrap: nowrap; gap: 20px; height: min-content; justify-content: flex-start; overflow: hidden; padding: 30px 10px 30px 30px; position: relative; width: 230px; will-change: var(--framer-will-change-override, transform); }',
  '.framer-i47EY .framer-1lgaaot, .framer-i47EY .framer-11gi34, .framer-i47EY .framer-136ha75, .framer-i47EY .framer-1tyoeu8, .framer-i47EY .framer-1wz2c6l, .framer-i47EY .framer-l7re06, .framer-i47EY .framer-1nzr4ba, .framer-i47EY .framer-kocp2l, .framer-i47EY .framer-5rkukc, .framer-i47EY .framer-1eek154, .framer-i47EY .framer-1yjee80, .framer-i47EY .framer-v4tks8, .framer-i47EY .framer-1flrnv1, .framer-i47EY .framer-oz1xw7, .framer-i47EY .framer-wqoxjf, .framer-i47EY .framer-1911m7a, .framer-i47EY .framer-17ciarf, .framer-i47EY .framer-1v49zt5, .framer-i47EY .framer-2nq4yf, .framer-i47EY .framer-1wg266f, .framer-i47EY .framer-1wb23t5, .framer-i47EY .framer-18gt7p0, .framer-i47EY .framer-i7k76n, .framer-i47EY .framer-fmfcsj, .framer-i47EY .framer-vcu553, .framer-i47EY .framer-12v1cfv, .framer-i47EY .framer-hedmwj, .framer-i47EY .framer-1856kn0, .framer-i47EY .framer-1oz1rxo, .framer-i47EY .framer-lad5xu, .framer-i47EY .framer-i71x2i, .framer-i47EY .framer-1jxboe2, .framer-i47EY .framer-1gayotb, .framer-i47EY .framer-tjuho, .framer-i47EY .framer-7enftg, .framer-i47EY .framer-zioe9h, .framer-i47EY .framer-9uhagv, .framer-i47EY .framer-1jfn7c, .framer-i47EY .framer-1pjiik1, .framer-i47EY .framer-qu0m4x, .framer-i47EY .framer-1ypvbg5, .framer-i47EY .framer-yelzwc, .framer-i47EY .framer-1y4hjz8, .framer-i47EY .framer-sneo2m, .framer-i47EY .framer-trwkm1, .framer-i47EY .framer-l4r5u9, .framer-i47EY .framer-1uivg47, .framer-i47EY .framer-x597qf, .framer-i47EY .framer-1cts9hh, .framer-i47EY .framer-yhha70, .framer-i47EY .framer-14p0usc, .framer-i47EY .framer-wdlvuy, .framer-i47EY .framer-1joqdpb, .framer-i47EY .framer-vdxi0, .framer-i47EY .framer-1wcks85, .framer-i47EY .framer-9b0t6j, .framer-i47EY .framer-1t94q5v, .framer-i47EY .framer-1hueufn, .framer-i47EY .framer-zaam3s, .framer-i47EY .framer-vtu735, .framer-i47EY .framer-18jtbce, .framer-i47EY .framer-1qo3ksb, .framer-i47EY .framer-99tywc, .framer-i47EY .framer-oisjv1, .framer-i47EY .framer-xw2o4k, .framer-i47EY .framer-1i5bydj, .framer-i47EY .framer-1edbi3b, .framer-i47EY .framer-1lq38p9, .framer-i47EY .framer-yi42xo, .framer-i47EY .framer-1v0nrzs, .framer-i47EY .framer-tfpd6e, .framer-i47EY .framer-1y1o55a, .framer-i47EY .framer-1g8j7hc, .framer-i47EY .framer-170zt9, .framer-i47EY .framer-tvg0jw, .framer-i47EY .framer-195sdz5, .framer-i47EY .framer-171k1td, .framer-i47EY .framer-1his063, .framer-i47EY .framer-tpbss6, .framer-i47EY .framer-x9ykqk, .framer-i47EY .framer-ff6fz9, .framer-i47EY .framer-1vee65n, .framer-i47EY .framer-68lhlv, .framer-i47EY .framer-14js4mt, .framer-i47EY .framer-5xzkik, .framer-i47EY .framer-zki8t0, .framer-i47EY .framer-1hslzhu, .framer-i47EY .framer-arkonl, .framer-i47EY .framer-tzreq4, .framer-i47EY .framer-1kp0bhg, .framer-i47EY .framer-1ptd02p, .framer-i47EY .framer-1cglloq, .framer-i47EY .framer-cecdzl, .framer-i47EY .framer-q8icxh, .framer-i47EY .framer-fjanm4, .framer-i47EY .framer-f1ay96, .framer-i47EY .framer-ur8tni, .framer-i47EY .framer-505x1r, .framer-i47EY .framer-1wms3xz, .framer-i47EY .framer-1csuvb5, .framer-i47EY .framer-pom0w, .framer-i47EY .framer-ke8bg0 { flex: none; height: auto; position: relative; white-space: pre; width: auto; }',
  '.framer-i47EY .framer-h9t7w7, .framer-i47EY .framer-nm4aj0, .framer-i47EY .framer-175pznh, .framer-i47EY .framer-2yntvv, .framer-i47EY .framer-1r68b23, .framer-i47EY .framer-17265uq, .framer-i47EY .framer-100vaex, .framer-i47EY .framer-1s8u59l { align-content: flex-end; align-items: flex-end; display: flex; flex: none; flex-direction: row; flex-wrap: nowrap; gap: 8px; height: min-content; justify-content: flex-start; overflow: visible; padding: 0px; position: relative; width: min-content; }',
  '.framer-i47EY .framer-2hibes, .framer-i47EY .framer-1oe7fs6, .framer-i47EY .framer-18p5km5, .framer-i47EY .framer-1ais9no, .framer-i47EY .framer-99vqpv, .framer-i47EY .framer-17ziua4, .framer-i47EY .framer-qn0o5u, .framer-i47EY .framer-94oxdl { flex: none; height: 48px; position: relative; white-space: pre; width: auto; }',
  '.framer-i47EY .framer-5yabb0, .framer-i47EY .framer-r4wr8c, .framer-i47EY .framer-pipxka, .framer-i47EY .framer-1nwrcuf, .framer-i47EY .framer-1h3ybkb, .framer-i47EY .framer-18995ll, .framer-i47EY .framer-1svare3, .framer-i47EY .framer-19vyt0w { align-content: center; align-items: center; display: flex; flex: none; flex-direction: column; flex-wrap: nowrap; gap: 10px; height: min-content; justify-content: flex-start; overflow: visible; padding: 20px 0px 20px 0px; position: relative; width: 100%; }',
  '.framer-i47EY .framer-axiu75, .framer-i47EY .framer-q5ntcw, .framer-i47EY .framer-1hb3cwx, .framer-i47EY .framer-z2gxu, .framer-i47EY .framer-bviehl, .framer-i47EY .framer-2wdsn5, .framer-i47EY .framer-l7p0ro, .framer-i47EY .framer-bznku0, .framer-i47EY .framer-3rb4qb, .framer-i47EY .framer-1odf1b2, .framer-i47EY .framer-111ge6c, .framer-i47EY .framer-vapxzp, .framer-i47EY .framer-czip0k, .framer-i47EY .framer-1jixfcb, .framer-i47EY .framer-gx8cy0, .framer-i47EY .framer-1lr6op5, .framer-i47EY .framer-1vfcu4x, .framer-i47EY .framer-14260yl, .framer-i47EY .framer-1kx0z0g, .framer-i47EY .framer-1lemtug, .framer-i47EY .framer-zd13hn, .framer-i47EY .framer-md98au, .framer-i47EY .framer-1uujxml, .framer-i47EY .framer-9kzb8d, .framer-i47EY .framer-1hazcdv, .framer-i47EY .framer-e5fm8s, .framer-i47EY .framer-r2m7ui, .framer-i47EY .framer-mpf025, .framer-i47EY .framer-mxjfz6, .framer-i47EY .framer-yafzz, .framer-i47EY .framer-w3cual, .framer-i47EY .framer-15vzggn { align-content: center; align-items: center; display: flex; flex: none; flex-direction: row; flex-wrap: nowrap; gap: 10px; height: min-content; justify-content: flex-start; overflow: visible; padding: 0px; position: relative; width: 100%; }',
  '.framer-i47EY .framer-7hjajm-container, .framer-i47EY .framer-rsyofj-container { flex: none; height: 15px; position: relative; width: 15px; }',
  '.framer-i47EY .framer-8v83xe, .framer-i47EY .framer-14onrej, .framer-i47EY .framer-1c7wwes, .framer-i47EY .framer-gdinzi, .framer-i47EY .framer-17vg6xx, .framer-i47EY .framer-chdw6h, .framer-i47EY .framer-1lbh0s6, .framer-i47EY .framer-1rypvf, .framer-i47EY .framer-1o4uz2h, .framer-i47EY .framer-1d5md58, .framer-i47EY .framer-dtblfb, .framer-i47EY .framer-1p3y9rt, .framer-i47EY .framer-1wsyad2, .framer-i47EY .framer-1himl70, .framer-i47EY .framer-q5ktmh, .framer-i47EY .framer-12q19vw, .framer-i47EY .framer-t0l3i6, .framer-i47EY .framer-1v34jr2, .framer-i47EY .framer-1d9mh93, .framer-i47EY .framer-1qx1tug, .framer-i47EY .framer-1bp6goi, .framer-i47EY .framer-iixjq0, .framer-i47EY .framer-xoy07e, .framer-i47EY .framer-dt7grq, .framer-i47EY .framer-ad17s4, .framer-i47EY .framer-14xvxp9, .framer-i47EY .framer-1xguxuj, .framer-i47EY .framer-17ro9bq, .framer-i47EY .framer-jq93oi, .framer-i47EY .framer-1tpv3ki { flex: none; height: 16px; position: relative; width: 16px; }',
  '.framer-i47EY .framer-1vsbw7d, .framer-i47EY .framer-1nh7kfl, .framer-i47EY .framer-1dlm72w, .framer-i47EY .framer-zgdo2p, .framer-i47EY .framer-o9sfrn, .framer-i47EY .framer-175vey9, .framer-i47EY .framer-138tvcp, .framer-i47EY .framer-m7btq5 { align-content: center; align-items: center; display: flex; flex: none; flex-direction: row; flex-wrap: nowrap; gap: 10px; height: 38px; justify-content: center; overflow: visible; padding: 15px; position: relative; width: min-content; }',
  '.framer-i47EY .framer-1s0vgn0, .framer-i47EY .framer-18fwcn, .framer-i47EY .framer-aww65b { align-content: center; align-items: center; display: flex; flex: none; flex-direction: column; flex-wrap: nowrap; gap: 10px; height: min-content; justify-content: center; overflow: hidden; padding: 0px 0px 100px 0px; position: relative; width: 1120px; }',
  '.framer-i47EY .framer-1sksdhj { align-content: center; align-items: center; display: flex; flex: none; flex-direction: column; flex-wrap: nowrap; gap: 12px; height: min-content; justify-content: center; overflow: hidden; padding: 0px; position: relative; width: 100%; }',
  '.framer-i47EY .framer-1hest51, .framer-i47EY .framer-d5yvmc, .framer-i47EY .framer-np9h7e { align-content: flex-start; align-items: flex-start; display: flex; flex: none; flex-direction: column; flex-wrap: nowrap; gap: 2px; height: min-content; justify-content: center; overflow: hidden; padding: 0px; position: relative; width: 100%; }',
  '.framer-i47EY .framer-1clngzw, .framer-i47EY .framer-921me1, .framer-i47EY .framer-a2l2lw { flex: none; height: auto; overflow: visible; position: relative; white-space: pre-wrap; width: 1120px; word-break: break-word; word-wrap: break-word; }',
  '.framer-i47EY .framer-k3iqma, .framer-i47EY .framer-1q72eta, .framer-i47EY .framer-f957eh { display: grid; flex: none; gap: 0px; grid-auto-rows: minmax(0, 1fr); grid-template-columns: repeat(5, minmax(50px, 1fr)); grid-template-rows: repeat(2, minmax(0, 1fr)); height: 317px; justify-content: center; overflow: hidden; padding: 0px; position: relative; width: 971px; }',
  '.framer-i47EY .framer-99xr8r, .framer-i47EY .framer-1xg4znl, .framer-i47EY .framer-1pdqso7 { align-content: flex-start; align-items: flex-start; align-self: center; display: flex; flex: none; flex-direction: row; flex-wrap: nowrap; gap: 10px; height: 100%; justify-content: center; justify-self: center; padding: 0px; position: relative; width: 100%; }',
  '.framer-i47EY .framer-1qnnkps, .framer-i47EY .framer-1qczvql, .framer-i47EY .framer-8czh2, .framer-i47EY .framer-5ya5nz, .framer-i47EY .framer-1ig55ua, .framer-i47EY .framer-1omxy4d, .framer-i47EY .framer-1gn92qy, .framer-i47EY .framer-ro7tfu, .framer-i47EY .framer-t1t9y9, .framer-i47EY .framer-1rsdhpb, .framer-i47EY .framer-1i23qxm, .framer-i47EY .framer-11k1b7b, .framer-i47EY .framer-1y0qryh, .framer-i47EY .framer-1aboik7, .framer-i47EY .framer-a82jx1, .framer-i47EY .framer-1ytxh3g, .framer-i47EY .framer-1ug839q, .framer-i47EY .framer-9qalg3, .framer-i47EY .framer-1pwsib5, .framer-i47EY .framer-5lpbh3, .framer-i47EY .framer-1d5vn6w, .framer-i47EY .framer-1n66xs4, .framer-i47EY .framer-myuomn, .framer-i47EY .framer-1905wxx, .framer-i47EY .framer-1xny62v, .framer-i47EY .framer-lxtiiw, .framer-i47EY .framer-ag1lnz, .framer-i47EY .framer-6gg80i, .framer-i47EY .framer-wop075, .framer-i47EY .framer-1qneiwc, .framer-i47EY .framer-1g20fq2, .framer-i47EY .framer-186yg3s, .framer-i47EY .framer-7v7ivb, .framer-i47EY .framer-1a8pwtg, .framer-i47EY .framer-7vbh0e, .framer-i47EY .framer-1tcucf3, .framer-i47EY .framer-gjmgpa, .framer-i47EY .framer-1t4ojrk, .framer-i47EY .framer-1v71s83, .framer-i47EY .framer-1r888j2, .framer-i47EY .framer-smhpgc, .framer-i47EY .framer-184xs2b, .framer-i47EY .framer-ee8a2w, .framer-i47EY .framer-1v4ghzi, .framer-i47EY .framer-1x73oz0, .framer-i47EY .framer-13yhtwp, .framer-i47EY .framer-m8nrn5, .framer-i47EY .framer-1xupaty, .framer-i47EY .framer-1hlpdp3, .framer-i47EY .framer-bxbveo, .framer-i47EY .framer-f79owq, .framer-i47EY .framer-10gvatr { align-content: center; align-items: center; align-self: center; display: flex; flex: none; flex-direction: row; flex-wrap: nowrap; gap: 10px; height: 100%; justify-content: center; justify-self: center; padding: 0px; position: relative; width: 100%; }',
  '.framer-i47EY .framer-1miy0c7, .framer-i47EY .framer-pc9xpx, .framer-i47EY .framer-dkecy8, .framer-i47EY .framer-1pvwsf4, .framer-i47EY .framer-nptynr, .framer-i47EY .framer-mfhbn0, .framer-i47EY .framer-lbq6cl, .framer-i47EY .framer-11g8lak, .framer-i47EY .framer-1vn6djv, .framer-i47EY .framer-bql8xh { align-self: start; flex: none; height: 100%; justify-self: start; position: relative; white-space: pre; width: 100%; }',
  '.framer-i47EY .framer-12vp7i5, .framer-i47EY .framer-ucfb0a { align-content: center; align-items: center; display: flex; flex: none; flex-direction: column; flex-wrap: nowrap; gap: 20px; height: min-content; justify-content: center; overflow: hidden; padding: 0px; position: relative; width: 100%; }',
  '.framer-i47EY .framer-188n9jw, .framer-i47EY .framer-1c9d5fk, .framer-i47EY .framer-e8wgrm, .framer-i47EY .framer-14gmvoy, .framer-i47EY .framer-sj2229, .framer-i47EY .framer-dbfywk, .framer-i47EY .framer-dr5idc, .framer-i47EY .framer-1frdxxx, .framer-i47EY .framer-1ph6jbd, .framer-i47EY .framer-1uc2uza, .framer-i47EY .framer-ngueq9, .framer-i47EY .framer-ua9zck, .framer-i47EY .framer-qwcsss { flex: none; height: 24px; position: relative; width: 24px; }',
  '.framer-i47EY .framer-qbpn7v, .framer-i47EY .framer-5w5ipt { flex: none; height: auto; position: relative; white-space: pre-wrap; width: 209px; word-break: break-word; word-wrap: break-word; }',
  '.framer-i47EY .framer-8f6pe6 { align-content: center; align-items: center; display: flex; flex: none; flex-direction: column; flex-wrap: nowrap; gap: 0px; height: 3415px; justify-content: flex-start; overflow: visible; padding: 60px 40px 60px 40px; position: relative; width: 320px; }',
  '.framer-i47EY .framer-9qs87f { flex: none; height: 96px; overflow: visible; position: relative; white-space: pre-wrap; width: 100%; word-break: break-word; word-wrap: break-word; }',
  '.framer-i47EY .framer-4q8dy1 { align-content: center; align-items: center; display: flex; flex: none; flex-direction: row; flex-wrap: nowrap; gap: 0px; height: min-content; justify-content: center; overflow: hidden; padding: 0px 0px 76px 0px; position: relative; width: 100%; }',
  '.framer-i47EY .framer-lc5uts { align-content: center; align-items: center; display: flex; flex: none; flex-direction: column; flex-wrap: nowrap; gap: 46px; height: min-content; justify-content: center; overflow: visible; padding: 0px; position: relative; width: 100%; }',
  '.framer-i47EY .framer-n2pkst-container, .framer-i47EY .framer-ii5715-container, .framer-i47EY .framer-hdkqhh-container { flex: none; height: auto; position: relative; width: 100%; }',
  '@supports (background: -webkit-named-image(i)) and (not (font-palette:dark)) { .framer-i47EY.framer-7simsb, .framer-i47EY .framer-q4z8b, .framer-i47EY .framer-1voyydo, .framer-i47EY .framer-lbd5da, .framer-i47EY .framer-1y83tpy, .framer-i47EY .framer-h9t7w7, .framer-i47EY .framer-5yabb0, .framer-i47EY .framer-axiu75, .framer-i47EY .framer-q5ntcw, .framer-i47EY .framer-1hb3cwx, .framer-i47EY .framer-z2gxu, .framer-i47EY .framer-1vsbw7d, .framer-i47EY .framer-1p01z0t, .framer-i47EY .framer-nm4aj0, .framer-i47EY .framer-r4wr8c, .framer-i47EY .framer-bviehl, .framer-i47EY .framer-2wdsn5, .framer-i47EY .framer-l7p0ro, .framer-i47EY .framer-bznku0, .framer-i47EY .framer-1nh7kfl, .framer-i47EY .framer-lal9la, .framer-i47EY .framer-175pznh, .framer-i47EY .framer-pipxka, .framer-i47EY .framer-3rb4qb, .framer-i47EY .framer-1odf1b2, .framer-i47EY .framer-111ge6c, .framer-i47EY .framer-vapxzp, .framer-i47EY .framer-1dlm72w, .framer-i47EY .framer-9qmgde, .framer-i47EY .framer-2yntvv, .framer-i47EY .framer-1nwrcuf, .framer-i47EY .framer-czip0k, .framer-i47EY .framer-1jixfcb, .framer-i47EY .framer-gx8cy0, .framer-i47EY .framer-1lr6op5, .framer-i47EY .framer-zgdo2p, .framer-i47EY .framer-1s0vgn0, .framer-i47EY .framer-1sksdhj, .framer-i47EY .framer-1hest51, .framer-i47EY .framer-99xr8r, .framer-i47EY .framer-1qnnkps, .framer-i47EY .framer-1qczvql, .framer-i47EY .framer-8czh2, .framer-i47EY .framer-5ya5nz, .framer-i47EY .framer-1ig55ua, .framer-i47EY .framer-1omxy4d, .framer-i47EY .framer-1gn92qy, .framer-i47EY .framer-ro7tfu, .framer-i47EY .framer-t1t9y9, .framer-i47EY .framer-1rsdhpb, .framer-i47EY .framer-1i23qxm, .framer-i47EY .framer-11k1b7b, .framer-i47EY .framer-1y0qryh, .framer-i47EY .framer-1aboik7, .framer-i47EY .framer-a82jx1, .framer-i47EY .framer-1ytxh3g, .framer-i47EY .framer-18fwcn, .framer-i47EY .framer-12vp7i5, .framer-i47EY .framer-d5yvmc, .framer-i47EY .framer-1xg4znl, .framer-i47EY .framer-1ug839q, .framer-i47EY .framer-9qalg3, .framer-i47EY .framer-1pwsib5, .framer-i47EY .framer-5lpbh3, .framer-i47EY .framer-1d5vn6w, .framer-i47EY .framer-1n66xs4, .framer-i47EY .framer-myuomn, .framer-i47EY .framer-1905wxx, .framer-i47EY .framer-1xny62v, .framer-i47EY .framer-lxtiiw, .framer-i47EY .framer-ag1lnz, .framer-i47EY .framer-6gg80i, .framer-i47EY .framer-wop075, .framer-i47EY .framer-1qneiwc, .framer-i47EY .framer-1g20fq2, .framer-i47EY .framer-186yg3s, .framer-i47EY .framer-aww65b, .framer-i47EY .framer-ucfb0a, .framer-i47EY .framer-np9h7e, .framer-i47EY .framer-1pdqso7, .framer-i47EY .framer-7v7ivb, .framer-i47EY .framer-1a8pwtg, .framer-i47EY .framer-7vbh0e, .framer-i47EY .framer-1tcucf3, .framer-i47EY .framer-gjmgpa, .framer-i47EY .framer-1t4ojrk, .framer-i47EY .framer-1v71s83, .framer-i47EY .framer-1r888j2, .framer-i47EY .framer-smhpgc, .framer-i47EY .framer-184xs2b, .framer-i47EY .framer-ee8a2w, .framer-i47EY .framer-1v4ghzi, .framer-i47EY .framer-1x73oz0, .framer-i47EY .framer-13yhtwp, .framer-i47EY .framer-m8nrn5, .framer-i47EY .framer-1xupaty, .framer-i47EY .framer-1hlpdp3, .framer-i47EY .framer-bxbveo, .framer-i47EY .framer-f79owq, .framer-i47EY .framer-10gvatr, .framer-i47EY .framer-8f6pe6, .framer-i47EY .framer-tlmrr2, .framer-i47EY .framer-4q8dy1, .framer-i47EY .framer-1i52f4v, .framer-i47EY .framer-mimhod, .framer-i47EY .framer-1r68b23, .framer-i47EY .framer-1h3ybkb, .framer-i47EY .framer-1vfcu4x, .framer-i47EY .framer-14260yl, .framer-i47EY .framer-1kx0z0g, .framer-i47EY .framer-1lemtug, .framer-i47EY .framer-o9sfrn, .framer-i47EY .framer-h4jq97, .framer-i47EY .framer-17265uq, .framer-i47EY .framer-18995ll, .framer-i47EY .framer-zd13hn, .framer-i47EY .framer-md98au, .framer-i47EY .framer-1uujxml, .framer-i47EY .framer-9kzb8d, .framer-i47EY .framer-175vey9, .framer-i47EY .framer-19e67rc, .framer-i47EY .framer-100vaex, .framer-i47EY .framer-1svare3, .framer-i47EY .framer-1hazcdv, .framer-i47EY .framer-e5fm8s, .framer-i47EY .framer-r2m7ui, .framer-i47EY .framer-mpf025, .framer-i47EY .framer-138tvcp, .framer-i47EY .framer-mrnxno, .framer-i47EY .framer-1s8u59l, .framer-i47EY .framer-19vyt0w, .framer-i47EY .framer-mxjfz6, .framer-i47EY .framer-yafzz, .framer-i47EY .framer-w3cual, .framer-i47EY .framer-15vzggn, .framer-i47EY .framer-m7btq5, .framer-i47EY .framer-lc5uts { gap: 0px; } .framer-i47EY.framer-7simsb > *, .framer-i47EY .framer-8f6pe6 > * { margin: 0px; margin-bottom: calc(0px / 2); margin-top: calc(0px / 2); } .framer-i47EY.framer-7simsb > :first-child, .framer-i47EY .framer-q4z8b > :first-child, .framer-i47EY .framer-1y83tpy > :first-child, .framer-i47EY .framer-5yabb0 > :first-child, .framer-i47EY .framer-1p01z0t > :first-child, .framer-i47EY .framer-r4wr8c > :first-child, .framer-i47EY .framer-lal9la > :first-child, .framer-i47EY .framer-pipxka > :first-child, .framer-i47EY .framer-9qmgde > :first-child, .framer-i47EY .framer-1nwrcuf > :first-child, .framer-i47EY .framer-1s0vgn0 > :first-child, .framer-i47EY .framer-1sksdhj > :first-child, .framer-i47EY .framer-1hest51 > :first-child, .framer-i47EY .framer-18fwcn > :first-child, .framer-i47EY .framer-12vp7i5 > :first-child, .framer-i47EY .framer-d5yvmc > :first-child, .framer-i47EY .framer-aww65b > :first-child, .framer-i47EY .framer-ucfb0a > :first-child, .framer-i47EY .framer-np9h7e > :first-child, .framer-i47EY .framer-8f6pe6 > :first-child, .framer-i47EY .framer-tlmrr2 > :first-child, .framer-i47EY .framer-mimhod > :first-child, .framer-i47EY .framer-1h3ybkb > :first-child, .framer-i47EY .framer-h4jq97 > :first-child, .framer-i47EY .framer-18995ll > :first-child, .framer-i47EY .framer-19e67rc > :first-child, .framer-i47EY .framer-1svare3 > :first-child, .framer-i47EY .framer-mrnxno > :first-child, .framer-i47EY .framer-19vyt0w > :first-child, .framer-i47EY .framer-lc5uts > :first-child { margin-top: 0px; } .framer-i47EY.framer-7simsb > :last-child, .framer-i47EY .framer-q4z8b > :last-child, .framer-i47EY .framer-1y83tpy > :last-child, .framer-i47EY .framer-5yabb0 > :last-child, .framer-i47EY .framer-1p01z0t > :last-child, .framer-i47EY .framer-r4wr8c > :last-child, .framer-i47EY .framer-lal9la > :last-child, .framer-i47EY .framer-pipxka > :last-child, .framer-i47EY .framer-9qmgde > :last-child, .framer-i47EY .framer-1nwrcuf > :last-child, .framer-i47EY .framer-1s0vgn0 > :last-child, .framer-i47EY .framer-1sksdhj > :last-child, .framer-i47EY .framer-1hest51 > :last-child, .framer-i47EY .framer-18fwcn > :last-child, .framer-i47EY .framer-12vp7i5 > :last-child, .framer-i47EY .framer-d5yvmc > :last-child, .framer-i47EY .framer-aww65b > :last-child, .framer-i47EY .framer-ucfb0a > :last-child, .framer-i47EY .framer-np9h7e > :last-child, .framer-i47EY .framer-8f6pe6 > :last-child, .framer-i47EY .framer-tlmrr2 > :last-child, .framer-i47EY .framer-mimhod > :last-child, .framer-i47EY .framer-1h3ybkb > :last-child, .framer-i47EY .framer-h4jq97 > :last-child, .framer-i47EY .framer-18995ll > :last-child, .framer-i47EY .framer-19e67rc > :last-child, .framer-i47EY .framer-1svare3 > :last-child, .framer-i47EY .framer-mrnxno > :last-child, .framer-i47EY .framer-19vyt0w > :last-child, .framer-i47EY .framer-lc5uts > :last-child { margin-bottom: 0px; } .framer-i47EY .framer-q4z8b > *, .framer-i47EY .framer-5yabb0 > *, .framer-i47EY .framer-r4wr8c > *, .framer-i47EY .framer-pipxka > *, .framer-i47EY .framer-1nwrcuf > *, .framer-i47EY .framer-1s0vgn0 > *, .framer-i47EY .framer-18fwcn > *, .framer-i47EY .framer-aww65b > *, .framer-i47EY .framer-tlmrr2 > *, .framer-i47EY .framer-1h3ybkb > *, .framer-i47EY .framer-18995ll > *, .framer-i47EY .framer-1svare3 > *, .framer-i47EY .framer-19vyt0w > * { margin: 0px; margin-bottom: calc(10px / 2); margin-top: calc(10px / 2); } .framer-i47EY .framer-1voyydo > *, .framer-i47EY .framer-4q8dy1 > * { margin: 0px; margin-left: calc(0px / 2); margin-right: calc(0px / 2); } .framer-i47EY .framer-1voyydo > :first-child, .framer-i47EY .framer-lbd5da > :first-child, .framer-i47EY .framer-h9t7w7 > :first-child, .framer-i47EY .framer-axiu75 > :first-child, .framer-i47EY .framer-q5ntcw > :first-child, .framer-i47EY .framer-1hb3cwx > :first-child, .framer-i47EY .framer-z2gxu > :first-child, .framer-i47EY .framer-1vsbw7d > :first-child, .framer-i47EY .framer-nm4aj0 > :first-child, .framer-i47EY .framer-bviehl > :first-child, .framer-i47EY .framer-2wdsn5 > :first-child, .framer-i47EY .framer-l7p0ro > :first-child, .framer-i47EY .framer-bznku0 > :first-child, .framer-i47EY .framer-1nh7kfl > :first-child, .framer-i47EY .framer-175pznh > :first-child, .framer-i47EY .framer-3rb4qb > :first-child, .framer-i47EY .framer-1odf1b2 > :first-child, .framer-i47EY .framer-111ge6c > :first-child, .framer-i47EY .framer-vapxzp > :first-child, .framer-i47EY .framer-1dlm72w > :first-child, .framer-i47EY .framer-2yntvv > :first-child, .framer-i47EY .framer-czip0k > :first-child, .framer-i47EY .framer-1jixfcb > :first-child, .framer-i47EY .framer-gx8cy0 > :first-child, .framer-i47EY .framer-1lr6op5 > :first-child, .framer-i47EY .framer-zgdo2p > :first-child, .framer-i47EY .framer-99xr8r > :first-child, .framer-i47EY .framer-1qnnkps > :first-child, .framer-i47EY .framer-1qczvql > :first-child, .framer-i47EY .framer-8czh2 > :first-child, .framer-i47EY .framer-5ya5nz > :first-child, .framer-i47EY .framer-1ig55ua > :first-child, .framer-i47EY .framer-1omxy4d > :first-child, .framer-i47EY .framer-1gn92qy > :first-child, .framer-i47EY .framer-ro7tfu > :first-child, .framer-i47EY .framer-t1t9y9 > :first-child, .framer-i47EY .framer-1rsdhpb > :first-child, .framer-i47EY .framer-1i23qxm > :first-child, .framer-i47EY .framer-11k1b7b > :first-child, .framer-i47EY .framer-1y0qryh > :first-child, .framer-i47EY .framer-1aboik7 > :first-child, .framer-i47EY .framer-a82jx1 > :first-child, .framer-i47EY .framer-1ytxh3g > :first-child, .framer-i47EY .framer-1xg4znl > :first-child, .framer-i47EY .framer-1ug839q > :first-child, .framer-i47EY .framer-9qalg3 > :first-child, .framer-i47EY .framer-1pwsib5 > :first-child, .framer-i47EY .framer-5lpbh3 > :first-child, .framer-i47EY .framer-1d5vn6w > :first-child, .framer-i47EY .framer-1n66xs4 > :first-child, .framer-i47EY .framer-myuomn > :first-child, .framer-i47EY .framer-1905wxx > :first-child, .framer-i47EY .framer-1xny62v > :first-child, .framer-i47EY .framer-lxtiiw > :first-child, .framer-i47EY .framer-ag1lnz > :first-child, .framer-i47EY .framer-6gg80i > :first-child, .framer-i47EY .framer-wop075 > :first-child, .framer-i47EY .framer-1qneiwc > :first-child, .framer-i47EY .framer-1g20fq2 > :first-child, .framer-i47EY .framer-186yg3s > :first-child, .framer-i47EY .framer-1pdqso7 > :first-child, .framer-i47EY .framer-7v7ivb > :first-child, .framer-i47EY .framer-1a8pwtg > :first-child, .framer-i47EY .framer-7vbh0e > :first-child, .framer-i47EY .framer-1tcucf3 > :first-child, .framer-i47EY .framer-gjmgpa > :first-child, .framer-i47EY .framer-1t4ojrk > :first-child, .framer-i47EY .framer-1v71s83 > :first-child, .framer-i47EY .framer-1r888j2 > :first-child, .framer-i47EY .framer-smhpgc > :first-child, .framer-i47EY .framer-184xs2b > :first-child, .framer-i47EY .framer-ee8a2w > :first-child, .framer-i47EY .framer-1v4ghzi > :first-child, .framer-i47EY .framer-1x73oz0 > :first-child, .framer-i47EY .framer-13yhtwp > :first-child, .framer-i47EY .framer-m8nrn5 > :first-child, .framer-i47EY .framer-1xupaty > :first-child, .framer-i47EY .framer-1hlpdp3 > :first-child, .framer-i47EY .framer-bxbveo > :first-child, .framer-i47EY .framer-f79owq > :first-child, .framer-i47EY .framer-10gvatr > :first-child, .framer-i47EY .framer-4q8dy1 > :first-child, .framer-i47EY .framer-1i52f4v > :first-child, .framer-i47EY .framer-1r68b23 > :first-child, .framer-i47EY .framer-1vfcu4x > :first-child, .framer-i47EY .framer-14260yl > :first-child, .framer-i47EY .framer-1kx0z0g > :first-child, .framer-i47EY .framer-1lemtug > :first-child, .framer-i47EY .framer-o9sfrn > :first-child, .framer-i47EY .framer-17265uq > :first-child, .framer-i47EY .framer-zd13hn > :first-child, .framer-i47EY .framer-md98au > :first-child, .framer-i47EY .framer-1uujxml > :first-child, .framer-i47EY .framer-9kzb8d > :first-child, .framer-i47EY .framer-175vey9 > :first-child, .framer-i47EY .framer-100vaex > :first-child, .framer-i47EY .framer-1hazcdv > :first-child, .framer-i47EY .framer-e5fm8s > :first-child, .framer-i47EY .framer-r2m7ui > :first-child, .framer-i47EY .framer-mpf025 > :first-child, .framer-i47EY .framer-138tvcp > :first-child, .framer-i47EY .framer-1s8u59l > :first-child, .framer-i47EY .framer-mxjfz6 > :first-child, .framer-i47EY .framer-yafzz > :first-child, .framer-i47EY .framer-w3cual > :first-child, .framer-i47EY .framer-15vzggn > :first-child, .framer-i47EY .framer-m7btq5 > :first-child { margin-left: 0px; } .framer-i47EY .framer-1voyydo > :last-child, .framer-i47EY .framer-lbd5da > :last-child, .framer-i47EY .framer-h9t7w7 > :last-child, .framer-i47EY .framer-axiu75 > :last-child, .framer-i47EY .framer-q5ntcw > :last-child, .framer-i47EY .framer-1hb3cwx > :last-child, .framer-i47EY .framer-z2gxu > :last-child, .framer-i47EY .framer-1vsbw7d > :last-child, .framer-i47EY .framer-nm4aj0 > :last-child, .framer-i47EY .framer-bviehl > :last-child, .framer-i47EY .framer-2wdsn5 > :last-child, .framer-i47EY .framer-l7p0ro > :last-child, .framer-i47EY .framer-bznku0 > :last-child, .framer-i47EY .framer-1nh7kfl > :last-child, .framer-i47EY .framer-175pznh > :last-child, .framer-i47EY .framer-3rb4qb > :last-child, .framer-i47EY .framer-1odf1b2 > :last-child, .framer-i47EY .framer-111ge6c > :last-child, .framer-i47EY .framer-vapxzp > :last-child, .framer-i47EY .framer-1dlm72w > :last-child, .framer-i47EY .framer-2yntvv > :last-child, .framer-i47EY .framer-czip0k > :last-child, .framer-i47EY .framer-1jixfcb > :last-child, .framer-i47EY .framer-gx8cy0 > :last-child, .framer-i47EY .framer-1lr6op5 > :last-child, .framer-i47EY .framer-zgdo2p > :last-child, .framer-i47EY .framer-99xr8r > :last-child, .framer-i47EY .framer-1qnnkps > :last-child, .framer-i47EY .framer-1qczvql > :last-child, .framer-i47EY .framer-8czh2 > :last-child, .framer-i47EY .framer-5ya5nz > :last-child, .framer-i47EY .framer-1ig55ua > :last-child, .framer-i47EY .framer-1omxy4d > :last-child, .framer-i47EY .framer-1gn92qy > :last-child, .framer-i47EY .framer-ro7tfu > :last-child, .framer-i47EY .framer-t1t9y9 > :last-child, .framer-i47EY .framer-1rsdhpb > :last-child, .framer-i47EY .framer-1i23qxm > :last-child, .framer-i47EY .framer-11k1b7b > :last-child, .framer-i47EY .framer-1y0qryh > :last-child, .framer-i47EY .framer-1aboik7 > :last-child, .framer-i47EY .framer-a82jx1 > :last-child, .framer-i47EY .framer-1ytxh3g > :last-child, .framer-i47EY .framer-1xg4znl > :last-child, .framer-i47EY .framer-1ug839q > :last-child, .framer-i47EY .framer-9qalg3 > :last-child, .framer-i47EY .framer-1pwsib5 > :last-child, .framer-i47EY .framer-5lpbh3 > :last-child, .framer-i47EY .framer-1d5vn6w > :last-child, .framer-i47EY .framer-1n66xs4 > :last-child, .framer-i47EY .framer-myuomn > :last-child, .framer-i47EY .framer-1905wxx > :last-child, .framer-i47EY .framer-1xny62v > :last-child, .framer-i47EY .framer-lxtiiw > :last-child, .framer-i47EY .framer-ag1lnz > :last-child, .framer-i47EY .framer-6gg80i > :last-child, .framer-i47EY .framer-wop075 > :last-child, .framer-i47EY .framer-1qneiwc > :last-child, .framer-i47EY .framer-1g20fq2 > :last-child, .framer-i47EY .framer-186yg3s > :last-child, .framer-i47EY .framer-1pdqso7 > :last-child, .framer-i47EY .framer-7v7ivb > :last-child, .framer-i47EY .framer-1a8pwtg > :last-child, .framer-i47EY .framer-7vbh0e > :last-child, .framer-i47EY .framer-1tcucf3 > :last-child, .framer-i47EY .framer-gjmgpa > :last-child, .framer-i47EY .framer-1t4ojrk > :last-child, .framer-i47EY .framer-1v71s83 > :last-child, .framer-i47EY .framer-1r888j2 > :last-child, .framer-i47EY .framer-smhpgc > :last-child, .framer-i47EY .framer-184xs2b > :last-child, .framer-i47EY .framer-ee8a2w > :last-child, .framer-i47EY .framer-1v4ghzi > :last-child, .framer-i47EY .framer-1x73oz0 > :last-child, .framer-i47EY .framer-13yhtwp > :last-child, .framer-i47EY .framer-m8nrn5 > :last-child, .framer-i47EY .framer-1xupaty > :last-child, .framer-i47EY .framer-1hlpdp3 > :last-child, .framer-i47EY .framer-bxbveo > :last-child, .framer-i47EY .framer-f79owq > :last-child, .framer-i47EY .framer-10gvatr > :last-child, .framer-i47EY .framer-4q8dy1 > :last-child, .framer-i47EY .framer-1i52f4v > :last-child, .framer-i47EY .framer-1r68b23 > :last-child, .framer-i47EY .framer-1vfcu4x > :last-child, .framer-i47EY .framer-14260yl > :last-child, .framer-i47EY .framer-1kx0z0g > :last-child, .framer-i47EY .framer-1lemtug > :last-child, .framer-i47EY .framer-o9sfrn > :last-child, .framer-i47EY .framer-17265uq > :last-child, .framer-i47EY .framer-zd13hn > :last-child, .framer-i47EY .framer-md98au > :last-child, .framer-i47EY .framer-1uujxml > :last-child, .framer-i47EY .framer-9kzb8d > :last-child, .framer-i47EY .framer-175vey9 > :last-child, .framer-i47EY .framer-100vaex > :last-child, .framer-i47EY .framer-1hazcdv > :last-child, .framer-i47EY .framer-e5fm8s > :last-child, .framer-i47EY .framer-r2m7ui > :last-child, .framer-i47EY .framer-mpf025 > :last-child, .framer-i47EY .framer-138tvcp > :last-child, .framer-i47EY .framer-1s8u59l > :last-child, .framer-i47EY .framer-mxjfz6 > :last-child, .framer-i47EY .framer-yafzz > :last-child, .framer-i47EY .framer-w3cual > :last-child, .framer-i47EY .framer-15vzggn > :last-child, .framer-i47EY .framer-m7btq5 > :last-child { margin-right: 0px; } .framer-i47EY .framer-lbd5da > *, .framer-i47EY .framer-1i52f4v > * { margin: 0px; margin-left: calc(15px / 2); margin-right: calc(15px / 2); } .framer-i47EY .framer-1y83tpy > *, .framer-i47EY .framer-1p01z0t > *, .framer-i47EY .framer-lal9la > *, .framer-i47EY .framer-9qmgde > *, .framer-i47EY .framer-12vp7i5 > *, .framer-i47EY .framer-ucfb0a > *, .framer-i47EY .framer-mimhod > *, .framer-i47EY .framer-h4jq97 > *, .framer-i47EY .framer-19e67rc > *, .framer-i47EY .framer-mrnxno > * { margin: 0px; margin-bottom: calc(20px / 2); margin-top: calc(20px / 2); } .framer-i47EY .framer-h9t7w7 > *, .framer-i47EY .framer-nm4aj0 > *, .framer-i47EY .framer-175pznh > *, .framer-i47EY .framer-2yntvv > *, .framer-i47EY .framer-1r68b23 > *, .framer-i47EY .framer-17265uq > *, .framer-i47EY .framer-100vaex > *, .framer-i47EY .framer-1s8u59l > * { margin: 0px; margin-left: calc(8px / 2); margin-right: calc(8px / 2); } .framer-i47EY .framer-axiu75 > *, .framer-i47EY .framer-q5ntcw > *, .framer-i47EY .framer-1hb3cwx > *, .framer-i47EY .framer-z2gxu > *, .framer-i47EY .framer-1vsbw7d > *, .framer-i47EY .framer-bviehl > *, .framer-i47EY .framer-2wdsn5 > *, .framer-i47EY .framer-l7p0ro > *, .framer-i47EY .framer-bznku0 > *, .framer-i47EY .framer-1nh7kfl > *, .framer-i47EY .framer-3rb4qb > *, .framer-i47EY .framer-1odf1b2 > *, .framer-i47EY .framer-111ge6c > *, .framer-i47EY .framer-vapxzp > *, .framer-i47EY .framer-1dlm72w > *, .framer-i47EY .framer-czip0k > *, .framer-i47EY .framer-1jixfcb > *, .framer-i47EY .framer-gx8cy0 > *, .framer-i47EY .framer-1lr6op5 > *, .framer-i47EY .framer-zgdo2p > *, .framer-i47EY .framer-99xr8r > *, .framer-i47EY .framer-1qnnkps > *, .framer-i47EY .framer-1qczvql > *, .framer-i47EY .framer-8czh2 > *, .framer-i47EY .framer-5ya5nz > *, .framer-i47EY .framer-1ig55ua > *, .framer-i47EY .framer-1omxy4d > *, .framer-i47EY .framer-1gn92qy > *, .framer-i47EY .framer-ro7tfu > *, .framer-i47EY .framer-t1t9y9 > *, .framer-i47EY .framer-1rsdhpb > *, .framer-i47EY .framer-1i23qxm > *, .framer-i47EY .framer-11k1b7b > *, .framer-i47EY .framer-1y0qryh > *, .framer-i47EY .framer-1aboik7 > *, .framer-i47EY .framer-a82jx1 > *, .framer-i47EY .framer-1ytxh3g > *, .framer-i47EY .framer-1xg4znl > *, .framer-i47EY .framer-1ug839q > *, .framer-i47EY .framer-9qalg3 > *, .framer-i47EY .framer-1pwsib5 > *, .framer-i47EY .framer-5lpbh3 > *, .framer-i47EY .framer-1d5vn6w > *, .framer-i47EY .framer-1n66xs4 > *, .framer-i47EY .framer-myuomn > *, .framer-i47EY .framer-1905wxx > *, .framer-i47EY .framer-1xny62v > *, .framer-i47EY .framer-lxtiiw > *, .framer-i47EY .framer-ag1lnz > *, .framer-i47EY .framer-6gg80i > *, .framer-i47EY .framer-wop075 > *, .framer-i47EY .framer-1qneiwc > *, .framer-i47EY .framer-1g20fq2 > *, .framer-i47EY .framer-186yg3s > *, .framer-i47EY .framer-1pdqso7 > *, .framer-i47EY .framer-7v7ivb > *, .framer-i47EY .framer-1a8pwtg > *, .framer-i47EY .framer-7vbh0e > *, .framer-i47EY .framer-1tcucf3 > *, .framer-i47EY .framer-gjmgpa > *, .framer-i47EY .framer-1t4ojrk > *, .framer-i47EY .framer-1v71s83 > *, .framer-i47EY .framer-1r888j2 > *, .framer-i47EY .framer-smhpgc > *, .framer-i47EY .framer-184xs2b > *, .framer-i47EY .framer-ee8a2w > *, .framer-i47EY .framer-1v4ghzi > *, .framer-i47EY .framer-1x73oz0 > *, .framer-i47EY .framer-13yhtwp > *, .framer-i47EY .framer-m8nrn5 > *, .framer-i47EY .framer-1xupaty > *, .framer-i47EY .framer-1hlpdp3 > *, .framer-i47EY .framer-bxbveo > *, .framer-i47EY .framer-f79owq > *, .framer-i47EY .framer-10gvatr > *, .framer-i47EY .framer-1vfcu4x > *, .framer-i47EY .framer-14260yl > *, .framer-i47EY .framer-1kx0z0g > *, .framer-i47EY .framer-1lemtug > *, .framer-i47EY .framer-o9sfrn > *, .framer-i47EY .framer-zd13hn > *, .framer-i47EY .framer-md98au > *, .framer-i47EY .framer-1uujxml > *, .framer-i47EY .framer-9kzb8d > *, .framer-i47EY .framer-175vey9 > *, .framer-i47EY .framer-1hazcdv > *, .framer-i47EY .framer-e5fm8s > *, .framer-i47EY .framer-r2m7ui > *, .framer-i47EY .framer-mpf025 > *, .framer-i47EY .framer-138tvcp > *, .framer-i47EY .framer-mxjfz6 > *, .framer-i47EY .framer-yafzz > *, .framer-i47EY .framer-w3cual > *, .framer-i47EY .framer-15vzggn > *, .framer-i47EY .framer-m7btq5 > * { margin: 0px; margin-left: calc(10px / 2); margin-right: calc(10px / 2); } .framer-i47EY .framer-1sksdhj > * { margin: 0px; margin-bottom: calc(12px / 2); margin-top: calc(12px / 2); } .framer-i47EY .framer-1hest51 > *, .framer-i47EY .framer-d5yvmc > *, .framer-i47EY .framer-np9h7e > * { margin: 0px; margin-bottom: calc(2px / 2); margin-top: calc(2px / 2); } .framer-i47EY .framer-lc5uts > * { margin: 0px; margin-bottom: calc(46px / 2); margin-top: calc(46px / 2); } }',
  '.framer-i47EY.framer-v-3f72b1.framer-7simsb { height: 2652px; width: 768px; }',
  '.framer-i47EY.framer-v-3f72b1 .framer-1voyydo { padding: 0px 0px 45px 0px; }',
  '.framer-i47EY.framer-v-3f72b1 .framer-1s0vgn0, .framer-i47EY.framer-v-3f72b1 .framer-k3iqma, .framer-i47EY.framer-v-3f72b1 .framer-18fwcn, .framer-i47EY.framer-v-3f72b1 .framer-921me1, .framer-i47EY.framer-v-3f72b1 .framer-1q72eta, .framer-i47EY.framer-v-3f72b1 .framer-aww65b, .framer-i47EY.framer-v-3f72b1 .framer-a2l2lw, .framer-i47EY.framer-v-3f72b1 .framer-f957eh { width: 100%; }',
  '.framer-i47EY.framer-v-3f72b1 .framer-1sksdhj { align-content: flex-start; align-items: flex-start; }',
  '.framer-i47EY.framer-v-3f72b1 .framer-1hest51 { justify-content: flex-start; width: min-content; }',
  '.framer-i47EY.framer-v-3f72b1 .framer-1clngzw { align-self: stretch; width: auto; }',
  '.framer-i47EY.framer-v-14tpty9.framer-7simsb { height: 3494px; padding: 60px 40px 60px 40px; width: 320px; }',
  ...css2,
  ...css,
  '.framer-i47EY[data-border="true"]::after, .framer-i47EY [data-border="true"]::after { content: ""; border-width: var(--border-top-width, 0) var(--border-right-width, 0) var(--border-bottom-width, 0) var(--border-left-width, 0); border-color: var(--border-color, none); border-style: var(--border-style, none); width: 100%; height: 100%; position: absolute; box-sizing: border-box; left: 0; top: 0; border-radius: inherit; pointer-events: none; }',
];
var FramerkTXE7wUBN = withCSS4(Component4, css6, 'framer-i47EY',);
var stdin_default4 = FramerkTXE7wUBN;
FramerkTXE7wUBN.displayName = 'Pricing (Page)';
FramerkTXE7wUBN.defaultProps = { height: 2526, width: 1200, };
addPropertyControls5(FramerkTXE7wUBN, {
  variant: {
    options: ['bAm7TcIeo', 'wm4kyLmlr', 'OlQQ934Vt',],
    optionTitles: ['Desktop', 'Tablet', 'phone',],
    title: 'Variant',
    type: ControlType5.Enum,
  },
},);
addFonts4(FramerkTXE7wUBN, [
  {
    explicitInter: true,
    fonts: [{
      family: 'Inter',
      source: 'framer',
      style: 'normal',
      unicodeRange: 'U+0460-052F, U+1C80-1C88, U+20B4, U+2DE0-2DFF, U+A640-A69F, U+FE2E-FE2F',
      url: 'https://framerusercontent.com/assets/5vvr9Vy74if2I6bQbJvbw7SY1pQ.woff2',
      weight: '400',
    }, {
      family: 'Inter',
      source: 'framer',
      style: 'normal',
      unicodeRange: 'U+0301, U+0400-045F, U+0490-0491, U+04B0-04B1, U+2116',
      url: 'https://framerusercontent.com/assets/EOr0mi4hNtlgWNn9if640EZzXCo.woff2',
      weight: '400',
    }, {
      family: 'Inter',
      source: 'framer',
      style: 'normal',
      unicodeRange: 'U+1F00-1FFF',
      url: 'https://framerusercontent.com/assets/Y9k9QrlZAqio88Klkmbd8VoMQc.woff2',
      weight: '400',
    }, {
      family: 'Inter',
      source: 'framer',
      style: 'normal',
      unicodeRange: 'U+0370-03FF',
      url: 'https://framerusercontent.com/assets/OYrD2tBIBPvoJXiIHnLoOXnY9M.woff2',
      weight: '400',
    }, {
      family: 'Inter',
      source: 'framer',
      style: 'normal',
      unicodeRange: 'U+0100-024F, U+0259, U+1E00-1EFF, U+2020, U+20A0-20AB, U+20AD-20CF, U+2113, U+2C60-2C7F, U+A720-A7FF',
      url: 'https://framerusercontent.com/assets/JeYwfuaPfZHQhEG8U5gtPDZ7WQ.woff2',
      weight: '400',
    }, {
      family: 'Inter',
      source: 'framer',
      style: 'normal',
      unicodeRange:
        'U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+2000-206F, U+2074, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD',
      url: 'https://framerusercontent.com/assets/vQyevYAyHtARFwPqUzQGpnDs.woff2',
      weight: '400',
    }, {
      family: 'Inter',
      source: 'framer',
      style: 'normal',
      unicodeRange: 'U+0102-0103, U+0110-0111, U+0128-0129, U+0168-0169, U+01A0-01A1, U+01AF-01B0, U+1EA0-1EF9, U+20AB',
      url: 'https://framerusercontent.com/assets/b6Y37FthZeALduNqHicBT6FutY.woff2',
      weight: '400',
    }, {
      family: 'Inter',
      source: 'google',
      style: 'normal',
      url: 'https://fonts.gstatic.com/s/inter/v18/UcCO3FwrK3iLTeHuS_nVMrMxCp50SjIw2boKoduKmMEVuGKYMZ1rib2Bg-4.woff2',
      weight: '600',
    }, {
      family: 'Inter',
      source: 'google',
      style: 'normal',
      url: 'https://fonts.gstatic.com/s/inter/v18/UcCO3FwrK3iLTeHuS_nVMrMxCp50SjIw2boKoduKmMEVuFuYMZ1rib2Bg-4.woff2',
      weight: '700',
    }, {
      family: 'Inter',
      source: 'framer',
      style: 'normal',
      unicodeRange: 'U+0460-052F, U+1C80-1C88, U+20B4, U+2DE0-2DFF, U+A640-A69F, U+FE2E-FE2F',
      url: 'https://framerusercontent.com/assets/DpPBYI0sL4fYLgAkX8KXOPVt7c.woff2',
      weight: '700',
    }, {
      family: 'Inter',
      source: 'framer',
      style: 'normal',
      unicodeRange: 'U+0301, U+0400-045F, U+0490-0491, U+04B0-04B1, U+2116',
      url: 'https://framerusercontent.com/assets/4RAEQdEOrcnDkhHiiCbJOw92Lk.woff2',
      weight: '700',
    }, {
      family: 'Inter',
      source: 'framer',
      style: 'normal',
      unicodeRange: 'U+1F00-1FFF',
      url: 'https://framerusercontent.com/assets/1K3W8DizY3v4emK8Mb08YHxTbs.woff2',
      weight: '700',
    }, {
      family: 'Inter',
      source: 'framer',
      style: 'normal',
      unicodeRange: 'U+0370-03FF',
      url: 'https://framerusercontent.com/assets/tUSCtfYVM1I1IchuyCwz9gDdQ.woff2',
      weight: '700',
    }, {
      family: 'Inter',
      source: 'framer',
      style: 'normal',
      unicodeRange: 'U+0100-024F, U+0259, U+1E00-1EFF, U+2020, U+20A0-20AB, U+20AD-20CF, U+2113, U+2C60-2C7F, U+A720-A7FF',
      url: 'https://framerusercontent.com/assets/VgYFWiwsAC5OYxAycRXXvhze58.woff2',
      weight: '700',
    }, {
      family: 'Inter',
      source: 'framer',
      style: 'normal',
      unicodeRange:
        'U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+2000-206F, U+2074, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD',
      url: 'https://framerusercontent.com/assets/DXD0Q7LSl7HEvDzucnyLnGBHM.woff2',
      weight: '700',
    }, {
      family: 'Inter',
      source: 'framer',
      style: 'normal',
      unicodeRange: 'U+0102-0103, U+0110-0111, U+0128-0129, U+0168-0169, U+01A0-01A1, U+01AF-01B0, U+1EA0-1EF9, U+20AB',
      url: 'https://framerusercontent.com/assets/GIryZETIX4IFypco5pYZONKhJIo.woff2',
      weight: '700',
    },],
  },
  ...FeatherFonts,
  ...MobilePlanFeaturesFonts,
  ...OnDemandMobileFonts,
  ...DedicatedMobileFonts,
  ...getFontsFromSharedStyle(fonts2,),
  ...getFontsFromSharedStyle(fonts,),
], { supportsExplicitInterCodegen: true, },);

// virtual:pricing
import { WithFramerBreakpoints, } from 'unframer';
import { jsx, } from 'react/jsx-runtime';
stdin_default4.Responsive = (props,) => {
  return /* @__PURE__ */ jsx(WithFramerBreakpoints, { Component: stdin_default4, ...props, },);
};
var pricing_default = stdin_default4;
export { pricing_default as default, };
