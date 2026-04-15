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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSunset.js
var require_faSunset = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSunset.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "sunset";
    var width = 576;
    var height = 512;
    var aliases = [127751];
    var unicode = "f767";
    var svgPathData = "M354.5 104.3L320 135.7 320 32c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 103.7-34.5-31.3c-13.1-11.9-33.3-10.9-45.2 2.2s-10.9 33.3 2.2 45.2l88 80c12.2 11.1 30.8 11.1 43.1 0l88-80c13.1-11.9 14-32.1 2.2-45.2s-32.1-14-45.2-2.2zm48.7 132.8c-1-5.3-4.6-9.8-9.6-11.9s-10.7-1.5-15.2 1.6L288 289.1l-90.3-62.3c-4.5-3.1-10.2-3.7-15.2-1.6s-8.6 6.6-9.6 11.9L153 345 45.1 364.9c-5.3 1-9.8 4.6-11.9 9.6s-1.5 10.7 1.6 15.2L64 432l105.3 0c19-46.9 65-80 118.7-80s99.7 33.1 118.7 80L512 432l29.2-42.3c3.1-4.5 3.7-10.2 1.6-15.2s-6.6-8.6-11.9-9.6L423 345 403.1 237.1zM0 488c0 13.3 10.7 24 24 24l528 0c13.3 0 24-10.7 24-24s-10.7-24-24-24L24 464c-13.3 0-24 10.7-24 24z";
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
    exports.faSunset = exports.definition;
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

// src/_entries/faSunset.js
var import_faSunset = __toESM(require_faSunset());
var export_faSunset = import_faSunset.definition;
export {
  export_faSunset as faSunset
};
