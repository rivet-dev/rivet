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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faShovelSnow.js
var require_faShovelSnow = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faShovelSnow.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "shovel-snow";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f7c3";
    var svgPathData = "M391 7c9.4-9.4 24.6-9.4 33.9 0l80 80c9.4 9.4 9.4 24.6 0 33.9l-31.4 31.4c-15.1 15.1-35.6 23.6-57 23.6c-13.5 0-26.2-3.3-37.4-9.2l-83.5 83.5 47 47c11.3 11.3 12.5 29.1 3 41.8l-120 160c-5.6 7.4-14.1 12.1-23.3 12.7s-18.3-2.7-24.9-9.3l-168-168C2.8 328.1-.6 319 .1 309.7s5.3-17.8 12.7-23.3l160-120c12.7-9.6 30.6-8.3 41.8 3l47 47 83.5-83.5c-5.9-11.2-9.2-23.9-9.2-37.4c0-21.4 8.5-41.9 23.6-57L391 7zm17 50.9L393.5 72.4c-6.1 6.1-9.5 14.4-9.5 23c0 18 14.6 32.6 32.6 32.6c8.6 0 16.9-3.4 23-9.5L454.1 104 408 57.9zM203.3 267.3c6.2-6.2 6.2-16.4 0-22.6s-16.4-6.2-22.6 0l-80 80c-6.2 6.2-6.2 16.4 0 22.6s16.4 6.2 22.6 0l80-80zm64 64c6.2-6.2 6.2-16.4 0-22.6s-16.4-6.2-22.6 0l-80 80c-6.2 6.2-6.2 16.4 0 22.6s16.4 6.2 22.6 0l80-80z";
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
    exports.faShovelSnow = exports.definition;
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

// src/_entries/faShovelSnow.js
var import_faShovelSnow = __toESM(require_faShovelSnow());
var export_faShovelSnow = import_faShovelSnow.definition;
export {
  export_faShovelSnow as faShovelSnow
};
