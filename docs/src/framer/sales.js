// @ts-nocheck
/* eslint-disable */
'use client';
import { className, className2, css, css2, fonts, fonts2, } from './chunk-76DSJYNX.js';
import './chunk-RUAN5HWR.js';

// https :https://framerusercontent.com/modules/ow6h1b6wQLBemft55pFs/4NSLp8g2XeiRa7UrhEvQ/XndMdburz.js
import { Fragment as _Fragment, jsx as _jsx2, jsxs as _jsxs2, } from 'react/jsx-runtime';
import {
  addFonts as addFonts2,
  ComponentViewportProvider,
  cx as cx2,
  FormContainer,
  FormPlainTextInput,
  getFonts,
  getFontsFromSharedStyle,
  Link,
  RichText as RichText2,
  SVG,
  useComponentViewport as useComponentViewport2,
  useLocaleInfo as useLocaleInfo2,
  useVariantState as useVariantState2,
  withCSS as withCSS2,
} from 'unframer';
import { LayoutGroup as LayoutGroup2, motion as motion2, MotionConfigContext as MotionConfigContext2, } from 'unframer';
import * as React2 from 'react';

// https :https://framerusercontent.com/modules/91AyXErLKVPpIjW2QRmt/w5BB3BLeNO1lUT62WiZh/JGT1yLjbN.js
import { jsx as _jsx, jsxs as _jsxs, } from 'react/jsx-runtime';
import {
  addFonts,
  addPropertyControls,
  ControlType,
  cx,
  RichText,
  useComponentViewport,
  useLocaleInfo,
  useVariantState,
  withCSS,
  withFX,
} from 'unframer';
import { LayoutGroup, motion, MotionConfigContext, } from 'unframer';
import * as React from 'react';
var MotionDivWithFX = withFX(motion.div,);
var enabledGestures = { Sc0Dg6vVo: { hover: true, pressed: true, }, };
var cycleOrder = ['Sc0Dg6vVo', 'eY3CsaRlv', 'Vt9zLruWe', 'dfgg9_FTh', 'MolWXut_w',];
var serializationHash = 'framer-FRLUR';
var variantClassNames = {
  dfgg9_FTh: 'framer-v-3zzmgb',
  eY3CsaRlv: 'framer-v-sqy5j3',
  MolWXut_w: 'framer-v-muutrj',
  Sc0Dg6vVo: 'framer-v-1v01vfs',
  Vt9zLruWe: 'framer-v-1r2hzz9',
};
function addPropertyOverrides(overrides, ...variants) {
  const nextOverrides = {};
  variants === null || variants === void 0
    ? void 0
    : variants.forEach((variant,) => variant && Object.assign(nextOverrides, overrides[variant],));
  return nextOverrides;
}
var transition1 = { delay: 0, duration: 0.2, ease: [0.44, 0, 0.56, 1,], type: 'tween', };
var transition2 = { delay: 0, duration: 1, ease: [0, 0, 1, 1,], type: 'tween', };
var animation = { opacity: 1, rotate: 360, rotateX: 0, rotateY: 0, scale: 1, skewX: 0, skewY: 0, x: 0, y: 0, };
var transformTemplate1 = (_, t,) => `translateX(-50%) ${t}`;
var Transition = ({ value, children, },) => {
  const config = React.useContext(MotionConfigContext,);
  const transition = value !== null && value !== void 0 ? value : config.transition;
  const contextValue = React.useMemo(() => ({ ...config, transition, }), [JSON.stringify(transition,),],);
  return /* @__PURE__ */ _jsx(MotionConfigContext.Provider, { value: contextValue, children, },);
};
var Variants = motion.create(React.Fragment,);
var humanReadableVariantMap = {
  Default: 'Sc0Dg6vVo',
  Disabled: 'Vt9zLruWe',
  Error: 'MolWXut_w',
  Loading: 'eY3CsaRlv',
  Success: 'dfgg9_FTh',
};
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
        : 'Sc0Dg6vVo',
  };
};
var createLayoutDependency = (props, variants,) => {
  if (props.layoutDependency) return variants.join('-',) + props.layoutDependency;
  return variants.join('-',);
};
var Component = /* @__PURE__ */ React.forwardRef(function (props, ref,) {
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
  } = useVariantState({ cycleOrder, defaultVariant: 'Sc0Dg6vVo', enabledGestures, variant, variantClassNames, },);
  const layoutDependency = createLayoutDependency(props, variants,);
  const ref1 = React.useRef(null,);
  const isDisplayed = () => {
    if (baseVariant === 'eY3CsaRlv') return false;
    return true;
  };
  const isDisplayed1 = () => {
    if (baseVariant === 'eY3CsaRlv') return true;
    return false;
  };
  const defaultLayoutId = React.useId();
  const sharedStyleClassNames = [];
  const componentViewport = useComponentViewport();
  return /* @__PURE__ */ _jsx(LayoutGroup, {
    id: layoutId !== null && layoutId !== void 0 ? layoutId : defaultLayoutId,
    children: /* @__PURE__ */ _jsx(Variants, {
      animate: variants,
      initial: false,
      children: /* @__PURE__ */ _jsx(Transition, {
        value: transition1,
        children: /* @__PURE__ */ _jsxs(motion.button, {
          ...restProps,
          ...gestureHandlers,
          className: cx(serializationHash, ...sharedStyleClassNames, 'framer-1v01vfs', className3, classNames,),
          'data-framer-name': 'Default',
          'data-reset': 'button',
          layoutDependency,
          layoutId: 'Sc0Dg6vVo',
          ref: ref !== null && ref !== void 0 ? ref : ref1,
          style: {
            backgroundColor: 'rgb(255, 79, 0)',
            borderBottomLeftRadius: 10,
            borderBottomRightRadius: 10,
            borderTopLeftRadius: 10,
            borderTopRightRadius: 10,
            opacity: 1,
            ...style,
          },
          variants: {
            'Sc0Dg6vVo-hover': { backgroundColor: 'rgba(255, 81, 0, 0.63)', },
            'Sc0Dg6vVo-pressed': { backgroundColor: 'rgba(255, 81, 0, 0.47)', },
            MolWXut_w: { backgroundColor: 'rgba(255, 34, 68, 0.15)', },
            Vt9zLruWe: { opacity: 0.5, },
          },
          ...addPropertyOverrides(
            {
              'Sc0Dg6vVo-hover': { 'data-framer-name': void 0, },
              'Sc0Dg6vVo-pressed': { 'data-framer-name': void 0, },
              dfgg9_FTh: { 'data-framer-name': 'Success', },
              eY3CsaRlv: { 'data-framer-name': 'Loading', },
              MolWXut_w: { 'data-framer-name': 'Error', },
              Vt9zLruWe: { 'data-framer-name': 'Disabled', },
            },
            baseVariant,
            gestureVariant,
          ),
          children: [
            isDisplayed() && /* @__PURE__ */ _jsx(RichText, {
              __fromCanvasComponent: true,
              children: /* @__PURE__ */ _jsx(React.Fragment, {
                children: /* @__PURE__ */ _jsx(motion.p, {
                  style: {
                    '--font-selector': 'SW50ZXItU2VtaUJvbGQ=',
                    '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                    '--framer-font-size': '14px',
                    '--framer-font-weight': '600',
                    '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                  },
                  children: 'Submit',
                },),
              },),
              className: 'framer-bagq5',
              fonts: ['Inter-SemiBold',],
              layoutDependency,
              layoutId: 'C3iA2_u4M',
              style: {
                '--extracted-r6o4lv': 'rgb(255, 255, 255)',
                '--framer-link-text-color': 'rgb(0, 153, 255)',
                '--framer-link-text-decoration': 'underline',
              },
              variants: { MolWXut_w: { '--extracted-r6o4lv': 'rgb(255, 34, 68)', }, },
              verticalAlignment: 'top',
              withExternalLayout: true,
              ...addPropertyOverrides(
                {
                  dfgg9_FTh: {
                    children: /* @__PURE__ */ _jsx(React.Fragment, {
                      children: /* @__PURE__ */ _jsx(motion.p, {
                        style: {
                          '--font-selector': 'SW50ZXItU2VtaUJvbGQ=',
                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                          '--framer-font-size': '14px',
                          '--framer-font-weight': '600',
                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 255, 255))',
                        },
                        children: 'Thank you',
                      },),
                    },),
                  },
                  MolWXut_w: {
                    children: /* @__PURE__ */ _jsx(React.Fragment, {
                      children: /* @__PURE__ */ _jsx(motion.p, {
                        style: {
                          '--font-selector': 'SW50ZXItU2VtaUJvbGQ=',
                          '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                          '--framer-font-size': '14px',
                          '--framer-font-weight': '600',
                          '--framer-text-color': 'var(--extracted-r6o4lv, rgb(255, 34, 68))',
                        },
                        children: 'Something went wrong',
                      },),
                    },),
                  },
                },
                baseVariant,
                gestureVariant,
              ),
            },),
            isDisplayed1() && /* @__PURE__ */ _jsx(motion.div, {
              className: 'framer-j7pn8o',
              'data-framer-name': 'Spinner',
              layoutDependency,
              layoutId: 'himpczHBK',
              style: {
                mask: 'url(\'https://framerusercontent.com/images/pGiXYozQ3mE4cilNOItfe2L2fUA.svg\') alpha no-repeat center / cover add',
                WebkitMask:
                  'url(\'https://framerusercontent.com/images/pGiXYozQ3mE4cilNOItfe2L2fUA.svg\') alpha no-repeat center / cover add',
              },
              children: /* @__PURE__ */ _jsx(MotionDivWithFX, {
                __framer__loop: animation,
                __framer__loopEffectEnabled: true,
                __framer__loopRepeatDelay: 0,
                __framer__loopRepeatType: 'loop',
                __framer__loopTransition: transition2,
                __perspectiveFX: false,
                __smartComponentFX: true,
                __targetOpacity: 1,
                className: 'framer-11aixt1',
                'data-framer-name': 'Conic',
                layoutDependency,
                layoutId: 'e_A3zaxgk',
                style: {
                  background: 'conic-gradient(from 180deg at 50% 50%, #4cf 0deg, #4cf 360deg)',
                  backgroundColor: 'rgb(68, 204, 255)',
                  mask: 'none',
                  WebkitMask: 'none',
                },
                variants: {
                  eY3CsaRlv: {
                    background:
                      'conic-gradient(from 0deg at 50% 50%, rgba(255, 255, 255, 0) 7.208614864864882deg, rgb(255, 255, 255) 342deg)',
                    backgroundColor: 'rgba(0, 0, 0, 0)',
                    mask:
                      'url(\'https://framerusercontent.com/images/pGiXYozQ3mE4cilNOItfe2L2fUA.svg\') alpha no-repeat center / cover add',
                    WebkitMask:
                      'url(\'https://framerusercontent.com/images/pGiXYozQ3mE4cilNOItfe2L2fUA.svg\') alpha no-repeat center / cover add',
                  },
                },
                children: /* @__PURE__ */ _jsx(motion.div, {
                  className: 'framer-1tle620',
                  'data-framer-name': 'Rounding',
                  layoutDependency,
                  layoutId: 'NXiKKM9np',
                  style: {
                    backgroundColor: 'rgb(255, 255, 255)',
                    borderBottomLeftRadius: 1,
                    borderBottomRightRadius: 1,
                    borderTopLeftRadius: 1,
                    borderTopRightRadius: 1,
                  },
                  transformTemplate: transformTemplate1,
                },),
              },),
            },),
          ],
        },),
      },),
    },),
  },);
},);
var css3 = [
  '@supports (aspect-ratio: 1) { body { --framer-aspect-ratio-supported: auto; } }',
  '.framer-FRLUR.framer-1rzbk8y, .framer-FRLUR .framer-1rzbk8y { display: block; }',
  '.framer-FRLUR.framer-1v01vfs { align-content: center; align-items: center; cursor: pointer; display: flex; flex-direction: row; flex-wrap: nowrap; gap: 0px; height: 40px; justify-content: center; overflow: visible; padding: 0px; position: relative; width: 240px; }',
  '.framer-FRLUR .framer-bagq5 { -webkit-user-select: none; flex: none; height: auto; position: relative; user-select: none; white-space: pre; width: auto; }',
  '.framer-FRLUR .framer-j7pn8o { aspect-ratio: 1 / 1; flex: none; height: var(--framer-aspect-ratio-supported, 20px); overflow: hidden; position: relative; width: 20px; }',
  '.framer-FRLUR .framer-11aixt1 { bottom: 0px; flex: none; left: 0px; overflow: visible; position: absolute; right: 0px; top: 0px; }',
  '.framer-FRLUR .framer-1tle620 { aspect-ratio: 1 / 1; flex: none; height: var(--framer-aspect-ratio-supported, 2px); left: 50%; overflow: visible; position: absolute; top: 0px; width: 2px; }',
  '@supports (background: -webkit-named-image(i)) and (not (font-palette:dark)) { .framer-FRLUR.framer-1v01vfs { gap: 0px; } .framer-FRLUR.framer-1v01vfs > * { margin: 0px; margin-left: calc(0px / 2); margin-right: calc(0px / 2); } .framer-FRLUR.framer-1v01vfs > :first-child { margin-left: 0px; } .framer-FRLUR.framer-1v01vfs > :last-child { margin-right: 0px; } }',
  '.framer-FRLUR.framer-v-sqy5j3.framer-1v01vfs, .framer-FRLUR.framer-v-1r2hzz9.framer-1v01vfs, .framer-FRLUR.framer-v-3zzmgb.framer-1v01vfs, .framer-FRLUR.framer-v-muutrj.framer-1v01vfs { cursor: unset; }',
  '.framer-FRLUR.framer-v-sqy5j3 .framer-11aixt1 { overflow: hidden; }',
];
var FramerJGT1yLjbN = withCSS(Component, css3, 'framer-FRLUR',);
var stdin_default = FramerJGT1yLjbN;
FramerJGT1yLjbN.displayName = 'Submit Button';
FramerJGT1yLjbN.defaultProps = { height: 40, width: 240, };
addPropertyControls(FramerJGT1yLjbN, {
  variant: {
    options: ['Sc0Dg6vVo', 'eY3CsaRlv', 'Vt9zLruWe', 'dfgg9_FTh', 'MolWXut_w',],
    optionTitles: ['Default', 'Loading', 'Disabled', 'Success', 'Error',],
    title: 'Variant',
    type: ControlType.Enum,
  },
},);
addFonts(FramerJGT1yLjbN, [{
  explicitInter: true,
  fonts: [{
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0460-052F, U+1C80-1C88, U+20B4, U+2DE0-2DFF, U+A640-A69F, U+FE2E-FE2F',
    url: 'https://framerusercontent.com/assets/hyOgCu0Xnghbimh0pE8QTvtt2AU.woff2',
    weight: '600',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0301, U+0400-045F, U+0490-0491, U+04B0-04B1, U+2116',
    url: 'https://framerusercontent.com/assets/NeGmSOXrPBfEFIy5YZeHq17LEDA.woff2',
    weight: '600',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+1F00-1FFF',
    url: 'https://framerusercontent.com/assets/oYaAX5himiTPYuN8vLWnqBbfD2s.woff2',
    weight: '600',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0370-03FF',
    url: 'https://framerusercontent.com/assets/lEJLP4R0yuCaMCjSXYHtJw72M.woff2',
    weight: '600',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0100-024F, U+0259, U+1E00-1EFF, U+2020, U+20A0-20AB, U+20AD-20CF, U+2113, U+2C60-2C7F, U+A720-A7FF',
    url: 'https://framerusercontent.com/assets/cRJyLNuTJR5jbyKzGi33wU9cqIQ.woff2',
    weight: '600',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange:
      'U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+2000-206F, U+2074, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD',
    url: 'https://framerusercontent.com/assets/1ZFS7N918ojhhd0nQWdj3jz4w.woff2',
    weight: '600',
  }, {
    family: 'Inter',
    source: 'framer',
    style: 'normal',
    unicodeRange: 'U+0102-0103, U+0110-0111, U+0128-0129, U+0168-0169, U+01A0-01A1, U+01AF-01B0, U+1EA0-1EF9, U+20AB',
    url: 'https://framerusercontent.com/assets/A0Wcc7NgXMjUuFdquHDrIZpzZw0.woff2',
    weight: '600',
  },],
},], { supportsExplicitInterCodegen: true, },);

// https :https://framerusercontent.com/modules/ow6h1b6wQLBemft55pFs/4NSLp8g2XeiRa7UrhEvQ/XndMdburz.js
var SubmitButtonFonts = getFonts(stdin_default,);
var serializationHash2 = 'framer-BK8wm';
var variantClassNames2 = { O8WVLLIhr: 'framer-v-1jjbo3t', };
var transition12 = { bounce: 0.2, delay: 0, duration: 0.4, type: 'spring', };
var formVariants = (form, variants, currentVariant,) => {
  switch (form.state) {
    case 'success':
      var _variants_success;
      return (_variants_success = variants.success) !== null && _variants_success !== void 0 ? _variants_success : currentVariant;
    case 'pending':
      var _variants_pending;
      return (_variants_pending = variants.pending) !== null && _variants_pending !== void 0 ? _variants_pending : currentVariant;
    case 'error':
      var _variants_error;
      return (_variants_error = variants.error) !== null && _variants_error !== void 0 ? _variants_error : currentVariant;
    case 'incomplete':
      var _variants_incomplete;
      return (_variants_incomplete = variants.incomplete) !== null && _variants_incomplete !== void 0
        ? _variants_incomplete
        : currentVariant;
  }
};
var Transition2 = ({ value, children, },) => {
  const config = React2.useContext(MotionConfigContext2,);
  const transition = value !== null && value !== void 0 ? value : config.transition;
  const contextValue = React2.useMemo(() => ({ ...config, transition, }), [JSON.stringify(transition,),],);
  return /* @__PURE__ */ _jsx2(MotionConfigContext2.Provider, { value: contextValue, children, },);
};
var Variants2 = motion2.create(React2.Fragment,);
var getProps2 = ({ height, id, width, ...props },) => {
  return { ...props, };
};
var createLayoutDependency2 = (props, variants,) => {
  if (props.layoutDependency) return variants.join('-',) + props.layoutDependency;
  return variants.join('-',);
};
var Component2 = /* @__PURE__ */ React2.forwardRef(function (props, ref,) {
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
  } = useVariantState2({ defaultVariant: 'O8WVLLIhr', variant, variantClassNames: variantClassNames2, },);
  const layoutDependency = createLayoutDependency2(props, variants,);
  const ref1 = React2.useRef(null,);
  const defaultLayoutId = React2.useId();
  const sharedStyleClassNames = [className2, className,];
  const componentViewport = useComponentViewport2();
  return /* @__PURE__ */ _jsx2(LayoutGroup2, {
    id: layoutId !== null && layoutId !== void 0 ? layoutId : defaultLayoutId,
    children: /* @__PURE__ */ _jsx2(Variants2, {
      animate: variants,
      initial: false,
      children: /* @__PURE__ */ _jsx2(Transition2, {
        value: transition12,
        children: /* @__PURE__ */ _jsxs2(motion2.div, {
          ...restProps,
          ...gestureHandlers,
          className: cx2(serializationHash2, ...sharedStyleClassNames, 'framer-1jjbo3t', className3, classNames,),
          'data-framer-name': 'Variant 1',
          layoutDependency,
          layoutId: 'O8WVLLIhr',
          ref: ref !== null && ref !== void 0 ? ref : ref1,
          style: { ...style, },
          children: [
            /* @__PURE__ */ _jsxs2(motion2.div, {
              className: 'framer-1v1ta5w',
              layoutDependency,
              layoutId: 'wgy60thcI',
              children: [
                /* @__PURE__ */ _jsxs2(motion2.div, {
                  className: 'framer-oe8dn1',
                  layoutDependency,
                  layoutId: 'Hno0cIgYo',
                  children: [
                    /* @__PURE__ */ _jsx2(RichText2, {
                      __fromCanvasComponent: true,
                      children: /* @__PURE__ */ _jsx2(React2.Fragment, {
                        children: /* @__PURE__ */ _jsx2(motion2.h3, {
                          className: 'framer-styles-preset-jttjmp',
                          'data-styles-preset': 'zu841OiIg',
                          children: 'Contact our Sales Team.',
                        },),
                      },),
                      className: 'framer-1m96p32',
                      fonts: ['Inter',],
                      layoutDependency,
                      layoutId: 'ZSgKNhLIw',
                      style: {
                        '--framer-link-text-color': 'rgb(0, 153, 255)',
                        '--framer-link-text-decoration': 'underline',
                        '--framer-paragraph-spacing': '0px',
                      },
                      verticalAlignment: 'top',
                      withExternalLayout: true,
                    },),
                    /* @__PURE__ */ _jsx2(RichText2, {
                      __fromCanvasComponent: true,
                      children: /* @__PURE__ */ _jsx2(React2.Fragment, {
                        children: /* @__PURE__ */ _jsx2(motion2.h2, {
                          className: 'framer-styles-preset-az499w',
                          'data-styles-preset': 'kHb0JRZSH',
                          children: 'Get a demo, tailored pricing to fit your needs, or have your questions answered about Rivet.',
                        },),
                      },),
                      className: 'framer-10s5a9w',
                      fonts: ['Inter',],
                      layoutDependency,
                      layoutId: 'bg3XzG9rS',
                      style: { '--framer-paragraph-spacing': '0px', },
                      verticalAlignment: 'top',
                      withExternalLayout: true,
                    },),
                  ],
                },),
                /* @__PURE__ */ _jsxs2(motion2.div, {
                  className: 'framer-15zinc2',
                  layoutDependency,
                  layoutId: 'KNfHfMqBr',
                  children: [
                    /* @__PURE__ */ _jsx2(RichText2, {
                      __fromCanvasComponent: true,
                      children: /* @__PURE__ */ _jsx2(React2.Fragment, {
                        children: /* @__PURE__ */ _jsx2(motion2.h2, {
                          className: 'framer-styles-preset-az499w',
                          'data-styles-preset': 'kHb0JRZSH',
                          style: { '--framer-text-color': 'var(--extracted-1of0zx5, rgb(255, 255, 255))', },
                          children: 'Email Support',
                        },),
                      },),
                      className: 'framer-rgl66s',
                      fonts: ['Inter',],
                      layoutDependency,
                      layoutId: 'PCNVeIHLM',
                      style: { '--extracted-1of0zx5': 'rgb(255, 255, 255)', '--framer-paragraph-spacing': '0px', },
                      verticalAlignment: 'top',
                      withExternalLayout: true,
                    },),
                    /* @__PURE__ */ _jsx2(RichText2, {
                      __fromCanvasComponent: true,
                      children: /* @__PURE__ */ _jsx2(React2.Fragment, {
                        children: /* @__PURE__ */ _jsx2(motion2.h2, {
                          className: 'framer-styles-preset-az499w',
                          'data-styles-preset': 'kHb0JRZSH',
                          children: 'support@rivet.gg',
                        },),
                      },),
                      className: 'framer-js62il',
                      fonts: ['Inter',],
                      layoutDependency,
                      layoutId: 'gD6EhkKso',
                      style: { '--framer-paragraph-spacing': '0px', },
                      verticalAlignment: 'top',
                      withExternalLayout: true,
                    },),
                  ],
                },),
                /* @__PURE__ */ _jsxs2(motion2.div, {
                  className: 'framer-10dl5v2',
                  layoutDependency,
                  layoutId: 'nNRFLRAaK',
                  children: [
                    /* @__PURE__ */ _jsx2(RichText2, {
                      __fromCanvasComponent: true,
                      children: /* @__PURE__ */ _jsx2(React2.Fragment, {
                        children: /* @__PURE__ */ _jsx2(motion2.h2, {
                          className: 'framer-styles-preset-az499w',
                          'data-styles-preset': 'kHb0JRZSH',
                          style: { '--framer-text-color': 'var(--extracted-1of0zx5, rgb(255, 255, 255))', },
                          children: 'Community Support',
                        },),
                      },),
                      className: 'framer-nacjtq',
                      fonts: ['Inter',],
                      layoutDependency,
                      layoutId: 'CkxJp4gie',
                      style: { '--extracted-1of0zx5': 'rgb(255, 255, 255)', '--framer-paragraph-spacing': '0px', },
                      verticalAlignment: 'top',
                      withExternalLayout: true,
                    },),
                    /* @__PURE__ */ _jsxs2(motion2.div, {
                      className: 'framer-t3i6b6',
                      layoutDependency,
                      layoutId: 'k56Ml9tCH',
                      children: [
                        /* @__PURE__ */ _jsx2(Link, {
                          href: 'https://discord.gg/rivet-developer-network-822914074136018994',
                          nodeId: 'NmAXr4NYQ',
                          children: /* @__PURE__ */ _jsx2(motion2.a, {
                            className: 'framer-1ylv83u framer-c8kegl',
                            layoutDependency,
                            layoutId: 'NmAXr4NYQ',
                            style: {
                              backgroundColor: 'rgb(255, 255, 255)',
                              borderBottomLeftRadius: 6,
                              borderBottomRightRadius: 6,
                              borderTopLeftRadius: 6,
                              borderTopRightRadius: 6,
                            },
                            children: /* @__PURE__ */ _jsx2(SVG, {
                              className: 'framer-16m23y7',
                              'data-framer-name': 'Discord',
                              layout: 'position',
                              layoutDependency,
                              layoutId: 'kA7mKUFZc',
                              opacity: 1,
                              svg:
                                '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 20 20"><path d="M 16.931 3.642 C 15.634 3.047 14.266 2.622 12.86 2.379 C 12.834 2.374 12.807 2.387 12.794 2.41 C 12.619 2.722 12.424 3.13 12.288 3.452 C 10.772 3.221 9.23 3.221 7.715 3.452 C 7.563 3.095 7.391 2.747 7.201 2.41 C 7.187 2.387 7.161 2.375 7.135 2.379 C 5.729 2.622 4.361 3.046 3.064 3.642 C 3.053 3.646 3.044 3.654 3.038 3.664 C 0.444 7.538 -0.267 11.317 0.083 15.047 C 0.084 15.066 0.094 15.083 0.108 15.095 C 1.618 16.214 3.307 17.067 5.103 17.62 C 5.128 17.628 5.156 17.618 5.173 17.597 C 5.559 17.072 5.9 16.516 6.194 15.935 C 6.202 15.919 6.203 15.9 6.197 15.883 C 6.19 15.866 6.177 15.853 6.16 15.847 C 5.621 15.64 5.1 15.392 4.6 15.103 C 4.581 15.093 4.569 15.073 4.568 15.052 C 4.567 15.03 4.576 15.01 4.593 14.997 C 4.699 14.918 4.802 14.837 4.903 14.753 C 4.921 14.739 4.946 14.735 4.968 14.745 C 8.241 16.239 11.784 16.239 15.019 14.745 C 15.041 14.735 15.066 14.738 15.084 14.753 C 15.184 14.835 15.289 14.918 15.395 14.997 C 15.412 15.009 15.421 15.03 15.42 15.051 C 15.419 15.072 15.408 15.091 15.39 15.102 C 14.892 15.394 14.369 15.642 13.829 15.846 C 13.812 15.852 13.799 15.866 13.792 15.883 C 13.786 15.9 13.787 15.919 13.795 15.935 C 14.095 16.517 14.438 17.07 14.816 17.596 C 14.832 17.618 14.86 17.627 14.886 17.619 C 16.685 17.068 18.376 16.214 19.888 15.094 C 19.902 15.084 19.912 15.067 19.914 15.049 C 20.331 10.735 19.216 6.987 16.957 3.666 C 16.951 3.655 16.942 3.646 16.931 3.641 Z M 6.683 12.775 C 5.697 12.775 4.886 11.871 4.886 10.759 C 4.886 9.648 5.682 8.743 6.683 8.743 C 7.692 8.743 8.497 9.657 8.481 10.76 C 8.481 11.871 7.684 12.775 6.683 12.775 Z M 13.329 12.775 C 12.343 12.775 11.532 11.871 11.532 10.759 C 11.532 9.648 12.328 8.743 13.329 8.743 C 14.338 8.743 15.143 9.657 15.127 10.76 C 15.127 11.871 14.338 12.775 13.329 12.775 Z" fill="rgb(0, 0, 0)"></path></svg>',
                              svgContentId: 9863990985,
                              withExternalLayout: true,
                            },),
                          },),
                        },),
                        /* @__PURE__ */ _jsx2(motion2.div, {
                          className: 'framer-8wj905',
                          layoutDependency,
                          layoutId: 'SzKMCPmsX',
                          style: {
                            backgroundColor: 'rgb(255, 255, 255)',
                            borderBottomLeftRadius: 6,
                            borderBottomRightRadius: 6,
                            borderTopLeftRadius: 6,
                            borderTopRightRadius: 6,
                          },
                          children: /* @__PURE__ */ _jsx2(SVG, {
                            className: 'framer-9ftf09',
                            'data-framer-name': 'Github',
                            layout: 'position',
                            layoutDependency,
                            layoutId: 'DZ05ABtRb',
                            opacity: 1,
                            svg:
                              '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 24 24"><path d="M 12 0.297 C 5.37 0.297 0 5.67 0 12.297 C 0 17.6 3.438 22.097 8.205 23.682 C 8.805 23.795 9.025 23.424 9.025 23.105 C 9.025 22.82 9.015 22.065 9.01 21.065 C 5.672 21.789 4.968 19.455 4.968 19.455 C 4.422 18.07 3.633 17.7 3.633 17.7 C 2.546 16.956 3.717 16.971 3.717 16.971 C 4.922 17.055 5.555 18.207 5.555 18.207 C 6.625 20.042 8.364 19.512 9.05 19.205 C 9.158 18.429 9.467 17.9 9.81 17.6 C 7.145 17.3 4.344 16.268 4.344 11.67 C 4.344 10.36 4.809 9.29 5.579 8.45 C 5.444 8.147 5.039 6.927 5.684 5.274 C 5.684 5.274 6.689 4.952 8.984 6.504 C 9.944 6.237 10.964 6.105 11.984 6.099 C 13.004 6.105 14.024 6.237 14.984 6.504 C 17.264 4.952 18.269 5.274 18.269 5.274 C 18.914 6.927 18.509 8.147 18.389 8.45 C 19.154 9.29 19.619 10.36 19.619 11.67 C 19.619 16.28 16.814 17.295 14.144 17.59 C 14.564 17.95 14.954 18.686 14.954 19.81 C 14.954 21.416 14.939 22.706 14.939 23.096 C 14.939 23.411 15.149 23.786 15.764 23.666 C 20.565 22.092 24 17.592 24 12.297 C 24 5.67 18.627 0.297 12 0.297" fill="rgb(0, 0, 0)"></path></svg>',
                            svgContentId: 9356778481,
                            withExternalLayout: true,
                          },),
                        },),
                      ],
                    },),
                  ],
                },),
              ],
            },),
            /* @__PURE__ */ _jsx2(FormContainer, {
              className: 'framer-1k5s7ql',
              layoutDependency,
              layoutId: 'H_v0_E3st',
              children: (formState,) =>
                /* @__PURE__ */ _jsxs2(_Fragment, {
                  children: [
                    /* @__PURE__ */ _jsxs2(motion2.label, {
                      className: 'framer-18b8b8o',
                      layoutDependency,
                      layoutId: 'Bt9K87jjT',
                      children: [
                        /* @__PURE__ */ _jsx2(RichText2, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx2(React2.Fragment, {
                            children: /* @__PURE__ */ _jsx2(motion2.p, {
                              style: {
                                '--font-selector': 'SW50ZXItTWVkaXVt',
                                '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                '--framer-font-size': '12px',
                                '--framer-font-weight': '500',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                              },
                              children: 'Email',
                            },),
                          },),
                          className: 'framer-499mo0',
                          fonts: ['Inter-Medium',],
                          layoutDependency,
                          layoutId: 'SHdD67Bgb',
                          style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                        /* @__PURE__ */ _jsx2(FormPlainTextInput, {
                          className: 'framer-hpyyb2',
                          inputName: 'Email',
                          layoutDependency,
                          layoutId: 'Q4lXvxc93',
                          placeholder: 'Email Address',
                          style: {
                            '--framer-input-background': 'rgba(187, 187, 187, 0.15)',
                            '--framer-input-border-bottom-width': '1px',
                            '--framer-input-border-color': 'rgba(136, 136, 136, 0.1)',
                            '--framer-input-border-left-width': '1px',
                            '--framer-input-border-radius-bottom-left': '10px',
                            '--framer-input-border-radius-bottom-right': '10px',
                            '--framer-input-border-radius-top-left': '10px',
                            '--framer-input-border-radius-top-right': '10px',
                            '--framer-input-border-right-width': '1px',
                            '--framer-input-border-style': 'solid',
                            '--framer-input-border-top-width': '1px',
                            '--framer-input-font-color': 'rgb(153, 153, 153)',
                            '--framer-input-icon-color': 'rgb(153, 153, 153)',
                            '--framer-input-placeholder-color': 'rgb(153, 153, 153)',
                          },
                          type: 'email',
                        },),
                      ],
                    },),
                    /* @__PURE__ */ _jsxs2(motion2.label, {
                      className: 'framer-18z0sf3',
                      layoutDependency,
                      layoutId: 'k3cxHIEme',
                      children: [
                        /* @__PURE__ */ _jsx2(RichText2, {
                          __fromCanvasComponent: true,
                          children: /* @__PURE__ */ _jsx2(React2.Fragment, {
                            children: /* @__PURE__ */ _jsx2(motion2.p, {
                              style: {
                                '--font-selector': 'SW50ZXItTWVkaXVt',
                                '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                                '--framer-font-size': '12px',
                                '--framer-font-weight': '500',
                                '--framer-text-color': 'var(--extracted-r6o4lv, rgb(136, 136, 136))',
                              },
                              children: 'How can we help?',
                            },),
                          },),
                          className: 'framer-135m04j',
                          fonts: ['Inter-Medium',],
                          layoutDependency,
                          layoutId: 'yuhYlqafz',
                          style: { '--extracted-r6o4lv': 'rgb(136, 136, 136)', },
                          verticalAlignment: 'top',
                          withExternalLayout: true,
                        },),
                        /* @__PURE__ */ _jsx2(FormPlainTextInput, {
                          className: 'framer-ziid8k',
                          inputName: 'Text',
                          layoutDependency,
                          layoutId: 'iD0sImSHJ',
                          placeholder: 'I would like Rivet to help solve for my company...',
                          required: true,
                          style: {
                            '--framer-input-background': 'rgba(187, 187, 187, 0.15)',
                            '--framer-input-border-bottom-width': '1px',
                            '--framer-input-border-color': 'rgba(136, 136, 136, 0.1)',
                            '--framer-input-border-left-width': '1px',
                            '--framer-input-border-radius-bottom-left': '10px',
                            '--framer-input-border-radius-bottom-right': '10px',
                            '--framer-input-border-radius-top-left': '10px',
                            '--framer-input-border-radius-top-right': '10px',
                            '--framer-input-border-right-width': '1px',
                            '--framer-input-border-style': 'solid',
                            '--framer-input-border-top-width': '1px',
                            '--framer-input-font-color': 'rgb(153, 153, 153)',
                            '--framer-input-icon-color': 'rgb(153, 153, 153)',
                            '--framer-input-placeholder-color': 'rgb(153, 153, 153)',
                          },
                          type: 'textarea',
                        },),
                      ],
                    },),
                    /* @__PURE__ */ _jsx2(ComponentViewportProvider, {
                      height: 40,
                      width: '334px',
                      children: /* @__PURE__ */ _jsx2(motion2.div, {
                        className: 'framer-1ff05f1-container',
                        layoutDependency,
                        layoutId: 'QJHd6hxCj-container',
                        children: /* @__PURE__ */ _jsx2(stdin_default, {
                          height: '100%',
                          id: 'QJHd6hxCj',
                          layoutId: 'QJHd6hxCj',
                          style: { height: '100%', width: '100%', },
                          type: 'submit',
                          variant: formVariants(formState, { pending: 'eY3CsaRlv', success: 'dfgg9_FTh', }, 'Sc0Dg6vVo',),
                          width: '100%',
                        },),
                      },),
                    },),
                    /* @__PURE__ */ _jsx2(RichText2, {
                      __fromCanvasComponent: true,
                      children: /* @__PURE__ */ _jsx2(React2.Fragment, {
                        children: /* @__PURE__ */ _jsxs2(motion2.h2, {
                          style: {
                            '--font-selector': 'SW50ZXItTWVkaXVt',
                            '--framer-font-family': '"Inter", "Inter Placeholder", sans-serif',
                            '--framer-font-weight': '500',
                            '--framer-letter-spacing': '-1px',
                            '--framer-text-alignment': 'left',
                            '--framer-text-color': 'var(--extracted-1of0zx5, rgb(138, 138, 138))',
                          },
                          children: [
                            'You can also email us at ',
                            /* @__PURE__ */ _jsx2(motion2.span, {
                              style: { '--font-selector': 'SW50ZXItQm9sZA==', '--framer-font-weight': '700', },
                              children: 'sales@rivet.gg',
                            },),
                          ],
                        },),
                      },),
                      className: 'framer-b5wt5e',
                      fonts: ['Inter-Medium', 'Inter-Bold',],
                      layoutDependency,
                      layoutId: 'F6HgtMCda',
                      style: { '--extracted-1of0zx5': 'rgb(138, 138, 138)', '--framer-paragraph-spacing': '0px', },
                      verticalAlignment: 'top',
                      withExternalLayout: true,
                    },),
                  ],
                },),
            },),
          ],
        },),
      },),
    },),
  },);
},);
var css4 = [
  '@supports (aspect-ratio: 1) { body { --framer-aspect-ratio-supported: auto; } }',
  '.framer-BK8wm.framer-c8kegl, .framer-BK8wm .framer-c8kegl { display: block; }',
  '.framer-BK8wm.framer-1jjbo3t { align-content: flex-start; align-items: flex-start; display: flex; flex-direction: row; flex-wrap: nowrap; gap: 166px; height: min-content; justify-content: center; overflow: hidden; padding: 0px; position: relative; width: 1200px; }',
  '.framer-BK8wm .framer-1v1ta5w { align-content: flex-start; align-items: flex-start; display: flex; flex: none; flex-direction: column; flex-wrap: nowrap; gap: 52px; height: min-content; justify-content: center; overflow: hidden; padding: 0px 0px 46px 0px; position: relative; width: 30%; }',
  '.framer-BK8wm .framer-oe8dn1, .framer-BK8wm .framer-15zinc2 { align-content: flex-start; align-items: flex-start; display: flex; flex: none; flex-direction: column; flex-wrap: nowrap; gap: 10px; height: min-content; justify-content: center; overflow: hidden; padding: 0px; position: relative; width: 100%; }',
  '.framer-BK8wm .framer-1m96p32 { flex: none; height: auto; overflow: visible; position: relative; white-space: pre-wrap; width: 354px; word-break: break-word; word-wrap: break-word; }',
  '.framer-BK8wm .framer-10s5a9w { flex: none; height: 79px; overflow: visible; position: relative; white-space: pre-wrap; width: 320px; word-break: break-word; word-wrap: break-word; }',
  '.framer-BK8wm .framer-rgl66s, .framer-BK8wm .framer-js62il, .framer-BK8wm .framer-nacjtq, .framer-BK8wm .framer-b5wt5e { flex: none; height: 25px; overflow: visible; position: relative; white-space: pre-wrap; width: 320px; word-break: break-word; word-wrap: break-word; }',
  '.framer-BK8wm .framer-10dl5v2 { align-content: flex-start; align-items: flex-start; display: flex; flex: none; flex-direction: column; flex-wrap: nowrap; gap: 10px; height: min-content; justify-content: flex-start; overflow: hidden; padding: 0px; position: relative; width: 100%; }',
  '.framer-BK8wm .framer-t3i6b6 { align-content: center; align-items: center; display: flex; flex: none; flex-direction: row; flex-wrap: nowrap; gap: 10px; height: min-content; justify-content: flex-start; overflow: hidden; padding: 0px; position: relative; width: 100%; }',
  '.framer-BK8wm .framer-1ylv83u { align-content: center; align-items: center; aspect-ratio: 1 / 1; display: flex; flex: none; flex-direction: row; flex-wrap: nowrap; gap: 10px; height: 31px; justify-content: center; overflow: hidden; padding: 0px; position: relative; text-decoration: none; width: var(--framer-aspect-ratio-supported, 31px); will-change: var(--framer-will-change-override, transform); }',
  '.framer-BK8wm .framer-16m23y7 { flex: none; height: 20px; position: relative; width: 20px; }',
  '.framer-BK8wm .framer-8wj905 { align-content: center; align-items: center; aspect-ratio: 1 / 1; display: flex; flex: none; flex-direction: row; flex-wrap: nowrap; gap: 10px; height: 31px; justify-content: center; overflow: hidden; padding: 0px; position: relative; width: var(--framer-aspect-ratio-supported, 31px); will-change: var(--framer-will-change-override, transform); }',
  '.framer-BK8wm .framer-9ftf09 { flex: none; height: 24px; position: relative; width: 24px; }',
  '.framer-BK8wm .framer-1k5s7ql { align-content: flex-start; align-items: flex-start; display: flex; flex: none; flex-direction: column; flex-wrap: nowrap; gap: 20px; height: min-content; justify-content: flex-start; overflow: hidden; padding: 0px 20px 20px 20px; position: relative; width: 374px; }',
  '.framer-BK8wm .framer-18b8b8o, .framer-BK8wm .framer-18z0sf3 { align-content: flex-start; align-items: flex-start; display: flex; flex: none; flex-direction: column; flex-wrap: nowrap; gap: 10px; height: min-content; justify-content: flex-start; padding: 0px; position: relative; width: 100%; }',
  '.framer-BK8wm .framer-499mo0, .framer-BK8wm .framer-135m04j { flex: none; height: auto; position: relative; white-space: pre; width: auto; }',
  '.framer-BK8wm .framer-hpyyb2 { --framer-input-focused-border-color: #0099ff; --framer-input-focused-border-style: solid; --framer-input-focused-border-width: 1px; --framer-input-font-family: "Inter"; --framer-input-font-letter-spacing: 0em; --framer-input-font-line-height: 1.2em; --framer-input-font-size: 14px; --framer-input-font-weight: 400; --framer-input-padding: 12px; flex: none; height: 40px; position: relative; width: 100%; }',
  '.framer-BK8wm .framer-ziid8k { --framer-input-focused-border-color: #0099ff; --framer-input-focused-border-style: solid; --framer-input-focused-border-width: 1px; --framer-input-font-family: "Inter"; --framer-input-font-letter-spacing: 0em; --framer-input-font-line-height: 1.2em; --framer-input-font-size: 14px; --framer-input-font-weight: 400; --framer-input-padding: 12px; --framer-input-wrapper-height: auto; --framer-textarea-resize: vertical; flex: none; height: auto; min-height: 173px; position: relative; width: 100%; }',
  '.framer-BK8wm .framer-1ff05f1-container { flex: none; height: 40px; position: relative; width: 100%; }',
  '@supports (background: -webkit-named-image(i)) and (not (font-palette:dark)) { .framer-BK8wm.framer-1jjbo3t, .framer-BK8wm .framer-1v1ta5w, .framer-BK8wm .framer-oe8dn1, .framer-BK8wm .framer-15zinc2, .framer-BK8wm .framer-10dl5v2, .framer-BK8wm .framer-t3i6b6, .framer-BK8wm .framer-1ylv83u, .framer-BK8wm .framer-8wj905, .framer-BK8wm .framer-1k5s7ql, .framer-BK8wm .framer-18b8b8o, .framer-BK8wm .framer-18z0sf3 { gap: 0px; } .framer-BK8wm.framer-1jjbo3t > * { margin: 0px; margin-left: calc(166px / 2); margin-right: calc(166px / 2); } .framer-BK8wm.framer-1jjbo3t > :first-child, .framer-BK8wm .framer-t3i6b6 > :first-child, .framer-BK8wm .framer-1ylv83u > :first-child, .framer-BK8wm .framer-8wj905 > :first-child { margin-left: 0px; } .framer-BK8wm.framer-1jjbo3t > :last-child, .framer-BK8wm .framer-t3i6b6 > :last-child, .framer-BK8wm .framer-1ylv83u > :last-child, .framer-BK8wm .framer-8wj905 > :last-child { margin-right: 0px; } .framer-BK8wm .framer-1v1ta5w > * { margin: 0px; margin-bottom: calc(52px / 2); margin-top: calc(52px / 2); } .framer-BK8wm .framer-1v1ta5w > :first-child, .framer-BK8wm .framer-oe8dn1 > :first-child, .framer-BK8wm .framer-15zinc2 > :first-child, .framer-BK8wm .framer-10dl5v2 > :first-child, .framer-BK8wm .framer-1k5s7ql > :first-child, .framer-BK8wm .framer-18b8b8o > :first-child, .framer-BK8wm .framer-18z0sf3 > :first-child { margin-top: 0px; } .framer-BK8wm .framer-1v1ta5w > :last-child, .framer-BK8wm .framer-oe8dn1 > :last-child, .framer-BK8wm .framer-15zinc2 > :last-child, .framer-BK8wm .framer-10dl5v2 > :last-child, .framer-BK8wm .framer-1k5s7ql > :last-child, .framer-BK8wm .framer-18b8b8o > :last-child, .framer-BK8wm .framer-18z0sf3 > :last-child { margin-bottom: 0px; } .framer-BK8wm .framer-oe8dn1 > *, .framer-BK8wm .framer-15zinc2 > *, .framer-BK8wm .framer-10dl5v2 > *, .framer-BK8wm .framer-18b8b8o > *, .framer-BK8wm .framer-18z0sf3 > * { margin: 0px; margin-bottom: calc(10px / 2); margin-top: calc(10px / 2); } .framer-BK8wm .framer-t3i6b6 > *, .framer-BK8wm .framer-1ylv83u > *, .framer-BK8wm .framer-8wj905 > * { margin: 0px; margin-left: calc(10px / 2); margin-right: calc(10px / 2); } .framer-BK8wm .framer-1k5s7ql > * { margin: 0px; margin-bottom: calc(20px / 2); margin-top: calc(20px / 2); } }',
  ...css2,
  ...css,
];
var FramerXndMdburz = withCSS2(Component2, css4, 'framer-BK8wm',);
var stdin_default2 = FramerXndMdburz;
FramerXndMdburz.displayName = 'Enterprise Support (Page)';
FramerXndMdburz.defaultProps = { height: 407, width: 1200, };
addFonts2(FramerXndMdburz, [
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
      source: 'framer',
      style: 'normal',
      unicodeRange: 'U+0460-052F, U+1C80-1C88, U+20B4, U+2DE0-2DFF, U+A640-A69F, U+FE2E-FE2F',
      url: 'https://framerusercontent.com/assets/5A3Ce6C9YYmCjpQx9M4inSaKU.woff2',
      weight: '500',
    }, {
      family: 'Inter',
      source: 'framer',
      style: 'normal',
      unicodeRange: 'U+0301, U+0400-045F, U+0490-0491, U+04B0-04B1, U+2116',
      url: 'https://framerusercontent.com/assets/Qx95Xyt0Ka3SGhinnbXIGpEIyP4.woff2',
      weight: '500',
    }, {
      family: 'Inter',
      source: 'framer',
      style: 'normal',
      unicodeRange: 'U+1F00-1FFF',
      url: 'https://framerusercontent.com/assets/6mJuEAguuIuMog10gGvH5d3cl8.woff2',
      weight: '500',
    }, {
      family: 'Inter',
      source: 'framer',
      style: 'normal',
      unicodeRange: 'U+0370-03FF',
      url: 'https://framerusercontent.com/assets/xYYWaj7wCU5zSQH0eXvSaS19wo.woff2',
      weight: '500',
    }, {
      family: 'Inter',
      source: 'framer',
      style: 'normal',
      unicodeRange: 'U+0100-024F, U+0259, U+1E00-1EFF, U+2020, U+20A0-20AB, U+20AD-20CF, U+2113, U+2C60-2C7F, U+A720-A7FF',
      url: 'https://framerusercontent.com/assets/otTaNuNpVK4RbdlT7zDDdKvQBA.woff2',
      weight: '500',
    }, {
      family: 'Inter',
      source: 'framer',
      style: 'normal',
      unicodeRange:
        'U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+2000-206F, U+2074, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD',
      url: 'https://framerusercontent.com/assets/d3tHnaQIAeqiE5hGcRw4mmgWYU.woff2',
      weight: '500',
    }, {
      family: 'Inter',
      source: 'framer',
      style: 'normal',
      unicodeRange: 'U+0102-0103, U+0110-0111, U+0128-0129, U+0168-0169, U+01A0-01A1, U+01AF-01B0, U+1EA0-1EF9, U+20AB',
      url: 'https://framerusercontent.com/assets/DolVirEGb34pEXEp8t8FQBSK4.woff2',
      weight: '500',
    }, {
      family: 'Inter',
      source: 'google',
      style: 'normal',
      url: 'https://fonts.gstatic.com/s/inter/v18/UcCO3FwrK3iLTeHuS_nVMrMxCp50SjIw2boKoduKmMEVuLyfMZ1rib2Bg-4.woff2',
      weight: '400',
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
  ...SubmitButtonFonts,
  ...getFontsFromSharedStyle(fonts2,),
  ...getFontsFromSharedStyle(fonts,),
], { supportsExplicitInterCodegen: true, },);

// virtual:sales
import { WithFramerBreakpoints, } from 'unframer';
import { jsx, } from 'react/jsx-runtime';
stdin_default2.Responsive = (props,) => {
  return /* @__PURE__ */ jsx(WithFramerBreakpoints, { Component: stdin_default2, ...props, },);
};
var sales_default = stdin_default2;
export { sales_default as default, };
