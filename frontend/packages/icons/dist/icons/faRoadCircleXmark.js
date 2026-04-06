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

// src/node_modules/@fortawesome/free-solid-svg-icons/faRoadCircleXmark.js
var require_faRoadCircleXmark = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faRoadCircleXmark.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "road-circle-xmark";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e566";
    var svgPathData = "M287.9 32l-76.2 0c-29.4 0-55.1 20.1-62.1 48.6L65.4 420.5C57.9 450.7 80.8 480 112 480l209.4 0c-11.2-24.4-17.4-51.4-17.4-80 0-28.6 6.2-55.7 17.4-80-.5 0-1 0-1.6 0-17.7 0-32-14.3-32-32l0-64c0-17.7 14.3-32 32-32s32 14.3 32 32l0 49.2c35.2-40 86.7-65.2 144.2-65.2 9 0 17.8 .6 26.4 1.8l-32-129.2C483.4 52.1 457.8 32 428.3 32l-76.5 0 0 64c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-64zM496 544a144 144 0 1 0 0-288 144 144 0 1 0 0 288zm59.3-180.7l-36.7 36.7 36.7 36.7c6.2 6.2 6.2 16.4 0 22.6s-16.4 6.2-22.6 0l-36.7-36.7-36.7 36.7c-6.2 6.2-16.4 6.2-22.6 0s-6.2-16.4 0-22.6l36.7-36.7-36.7-36.7c-6.2-6.2-6.2-16.4 0-22.6s16.4-6.2 22.6 0l36.7 36.7 36.7-36.7c6.2-6.2 16.4-6.2 22.6 0s6.2 16.4 0 22.6z";
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
    exports.faRoadCircleXmark = exports.definition;
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

// src/_entries/faRoadCircleXmark.js
var import_faRoadCircleXmark = __toESM(require_faRoadCircleXmark());
var export_faRoadCircleXmark = import_faRoadCircleXmark.definition;
export {
  export_faRoadCircleXmark as faRoadCircleXmark
};
