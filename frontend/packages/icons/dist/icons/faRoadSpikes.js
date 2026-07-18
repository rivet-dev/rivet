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

// src/node_modules/@fortawesome/free-solid-svg-icons/faRoadSpikes.js
var require_faRoadSpikes = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faRoadSpikes.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "road-spikes";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e568";
    var svgPathData = "M64 116.8c0-15.8 20.5-22 29.3-8.9l98.7 148 0-139.2c0-15.8 20.5-22 29.3-8.9l98.7 148 0-139.2c0-15.8 20.5-22 29.3-8.9l98.7 148 0-139.2c0-15.8 20.5-22 29.3-8.9L606.8 302.2c14.2 21.3-1.1 49.8-26.6 49.8L64 352 64 116.8zM32 384l576 0c17.7 0 32 14.3 32 32s-14.3 32-32 32L32 448c-17.7 0-32-14.3-32-32s14.3-32 32-32z";
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
    exports.faRoadSpikes = exports.definition;
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

// src/_entries/faRoadSpikes.js
var import_faRoadSpikes = __toESM(require_faRoadSpikes());
var export_faRoadSpikes = import_faRoadSpikes.definition;
export {
  export_faRoadSpikes as faRoadSpikes
};
