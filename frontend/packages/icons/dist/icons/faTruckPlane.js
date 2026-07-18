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

// src/node_modules/@fortawesome/free-solid-svg-icons/faTruckPlane.js
var require_faTruckPlane = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faTruckPlane.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "truck-plane";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e58f";
    var svgPathData = "M144 48c0-26.5 21.5-48 48-48s48 21.5 48 48l0 320c0 32.5 12.1 62.1 32 84.7l0 27.3c0 7.1 .9 14.1 2.7 20.7L192 480 83.9 507C73.8 509.6 64 501.9 64 491.5l0-19.8c0-4.9 2.2-9.5 6-12.5l74-59.2 0-57.6-122.1 48.8C11.4 395.4 0 387.7 0 376.4l0-44c0-8 3-15.6 8.3-21.5L144 161.6 144 48zm224 80l192 0c44.2 0 80 35.8 80 80l0 160c0 26.2-12.6 49.4-32 64l0 48c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-32-160 0 0 32c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-48c-19.4-14.6-32-37.8-32-64l0-160c0-44.2 35.8-80 80-80zm0 176l192 0 0-32c0-17.7-14.3-32-32-32l-128 0c-17.7 0-32 14.3-32 32l0 32zm0 88a24 24 0 1 0 0-48 24 24 0 1 0 0 48zm216-24a24 24 0 1 0 -48 0 24 24 0 1 0 48 0z";
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
    exports.faTruckPlane = exports.definition;
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

// src/_entries/faTruckPlane.js
var import_faTruckPlane = __toESM(require_faTruckPlane());
var export_faTruckPlane = import_faTruckPlane.definition;
export {
  export_faTruckPlane as faTruckPlane
};
