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

// src/node_modules/@fortawesome/free-solid-svg-icons/faRainbow.js
var require_faRainbow = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faRainbow.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "rainbow";
    var width = 640;
    var height = 512;
    var aliases = [127752];
    var unicode = "f75b";
    var svgPathData = "M320 96C178.6 96 64 210.6 64 352l0 64c0 17.7-14.3 32-32 32S0 433.7 0 416l0-64C0 175.3 143.3 32 320 32S640 175.3 640 352l0 64c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-64C576 210.6 461.4 96 320 96zm0 112c-79.5 0-144 64.5-144 144l0 64c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-64c0-114.9 93.1-208 208-208s208 93.1 208 208l0 64c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-64c0-79.5-64.5-144-144-144zM288 352l0 64c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-64c0-53 43-96 96-96s96 43 96 96l0 64c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-64c0-17.7-14.3-32-32-32s-32 14.3-32 32z";
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
    exports.faRainbow = exports.definition;
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

// src/_entries/faRainbow.js
var import_faRainbow = __toESM(require_faRainbow());
var export_faRainbow = import_faRainbow.definition;
export {
  export_faRainbow as faRainbow
};
