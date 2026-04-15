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

// src/node_modules/@fortawesome/free-solid-svg-icons/faTentArrowTurnLeft.js
var require_faTentArrowTurnLeft = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faTentArrowTurnLeft.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "tent-arrow-turn-left";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e580";
    var svgPathData = "M-.5 113l72 72c9.4 9.4 24.6 9.4 33.9 0s9.4-24.6 0-33.9l-31-31 350.1 0c22.1 0 40 17.9 40 40l0 24c0 13.3 10.7 24 24 24s24-10.7 24-24l0-24c0-48.6-39.4-88-88-88l-350.1 0 31-31c9.4-9.4 9.4-24.6 0-33.9S80.9-2.3 71.5 7L-.5 79c-9.4 9.4-9.4 24.6 0 33.9zM60.3 292.6L37.1 476c-2.4 19.1 12.5 36 31.7 36l154.5 0c17.7 0 32-14.3 32-32l0-117.4c0-5.9 4.7-10.6 10.6-10.6 4 0 7.7 2.3 9.5 5.9l68.2 136.5c5.4 10.8 16.5 17.7 28.6 17.7l71.9 0c19.3 0 34.2-16.9 31.7-36L452.7 292.7c-1.2-9.7-6.9-18.4-15.3-23.4L271.9 169.9c-10.2-6.1-22.9-6.1-33.1 .1L75.4 269.3c-8.3 5.1-13.9 13.7-15.1 23.3z";
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
    exports.faTentArrowTurnLeft = exports.definition;
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

// src/_entries/faTentArrowTurnLeft.js
var import_faTentArrowTurnLeft = __toESM(require_faTentArrowTurnLeft());
var export_faTentArrowTurnLeft = import_faTentArrowTurnLeft.definition;
export {
  export_faTentArrowTurnLeft as faTentArrowTurnLeft
};
