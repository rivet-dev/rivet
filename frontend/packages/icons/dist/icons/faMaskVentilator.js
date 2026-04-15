var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/free-solid-svg-icons/faMaskVentilator.js
var require_faMaskVentilator = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faMaskVentilator.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "mask-ventilator";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e524";
    var svgPathData = "M152.2 192c-15.4 38.3-24.2 77.2-24.2 108.8 0 13.3 1.5 26.2 4.4 38.7l-17-3.4c-29.9-6-51.4-32.2-51.4-62.8l0-81.3 88.2 0zm32.9-64L64 128c-35.3 0-64 28.7-64 64l0 81.3c0 61 43.1 113.5 102.9 125.5l66.3 13.3c18.7 22.1 42.9 40 70.8 51.8l0-87.8c-10.1-14.2-16-31.3-16-49.7 0-49.5 43-134.4 96-134.4 52.5 0 96 84.9 96 134.4 0 18.4-5.9 35.4-16 49.6l0 87.7c27.6-11.8 51.7-29.6 70.4-51.6l66.7-13.3c59.8-12 102.9-64.5 102.9-125.5l0-81.3c0-35.3-28.7-64-64-64l-121.5 0c-34.8-54.2-82.5-96-134.5-96-52.6 0-100.2 41.8-134.9 96zm302.5 64l88.4 0 0 81.3c0 30.5-21.5 56.8-51.4 62.8l-17 3.4c2.9-12.4 4.5-25.4 4.5-38.7 0-31.6-8.9-70.5-24.4-108.8zM288 320l0 192 64 0 0-192c0-17.7-14.3-32-32-32s-32 14.3-32 32z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faMaskVentilator = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faMaskVentilator.js
var import_faMaskVentilator = __toESM(require_faMaskVentilator());
var export_faMaskVentilator = import_faMaskVentilator.definition;
export {
  export_faMaskVentilator as faMaskVentilator
};
