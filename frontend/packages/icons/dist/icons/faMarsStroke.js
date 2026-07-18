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

// src/node_modules/@fortawesome/free-solid-svg-icons/faMarsStroke.js
var require_faMarsStroke = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faMarsStroke.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "mars-stroke";
    var width = 576;
    var height = 512;
    var aliases = [9894];
    var unicode = "f229";
    var svgPathData = "M416-32c-17.7 0-32 14.3-32 32s14.3 32 32 32l50.7 0-58.7 58.7-17.4-17.4c-12.5-12.5-32.8-12.5-45.3 0s-12.5 32.8 0 45.3l17.4 17.4-23 23c-28.4-19.5-62.7-31-99.8-31-97.2 0-176 78.8-176 176s78.8 176 176 176 176-78.8 176-176c0-37-11.4-71.4-31-99.8l23-23 17.4 17.4c12.5 12.5 32.8 12.5 45.3 0s12.5-32.8 0-45.3L453.3 136 512 77.3 512 128c0 17.7 14.3 32 32 32s32-14.3 32-32L576 0c0-17.7-14.3-32-32-32L416-32zM128 304a112 112 0 1 1 224 0 112 112 0 1 1 -224 0z";
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
    exports.faMarsStroke = exports.definition;
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

// src/_entries/faMarsStroke.js
var import_faMarsStroke = __toESM(require_faMarsStroke());
var export_faMarsStroke = import_faMarsStroke.definition;
export {
  export_faMarsStroke as faMarsStroke
};
