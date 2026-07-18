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

// src/node_modules/@fortawesome/free-solid-svg-icons/faTruckMoving.js
var require_faTruckMoving = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faTruckMoving.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "truck-moving";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f4df";
    var svgPathData = "M64 32C28.7 32 0 60.7 0 96L0 424c0 48.6 39.4 88 88 88 25.2 0 48-10.6 64-27.6 16 17 38.8 27.6 64 27.6 40.3 0 74.2-27.1 84.7-64l134.6 0c10.4 36.9 44.4 64 84.7 64 43 0 78.9-30.9 86.5-71.7 20-10.8 33.5-32 33.5-56.3l0-146.7c0-17-6.7-33.3-18.7-45.3L576 146.7c-12-12-28.3-18.7-45.3-18.7l-50.7 0 0-32c0-35.3-28.7-64-64-64L64 32zM480 192l50.7 0 45.3 45.3 0 50.7-96 0 0-96zM88 384a40 40 0 1 1 0 80 40 40 0 1 1 0-80zm392 40a40 40 0 1 1 80 0 40 40 0 1 1 -80 0zM216 384a40 40 0 1 1 0 80 40 40 0 1 1 0-80z";
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
    exports.faTruckMoving = exports.definition;
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

// src/_entries/faTruckMoving.js
var import_faTruckMoving = __toESM(require_faTruckMoving());
var export_faTruckMoving = import_faTruckMoving.definition;
export {
  export_faTruckMoving as faTruckMoving
};
