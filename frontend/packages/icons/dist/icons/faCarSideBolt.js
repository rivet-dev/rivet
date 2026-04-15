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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCarSideBolt.js
var require_faCarSideBolt = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCarSideBolt.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "car-side-bolt";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e344";
    var svgPathData = "M82.2 92.3L40.6 196.4C16.8 205.8 0 228.9 0 256L0 368c0 17.7 14.3 32 32 32l33.3 0c7.6 45.4 47.1 80 94.7 80s87.1-34.6 94.7-80l130.7 0c7.6 45.4 47.1 80 94.7 80s87.1-34.6 94.7-80l33.3 0c17.7 0 32-14.3 32-32l0-48c0-65.2-48.8-119-111.8-127L428.2 68c-18.2-22.8-45.8-36-75-36L171.3 32c-39.3 0-74.6 23.9-89.1 60.3zM480 336a48 48 0 1 1 0 96 48 48 0 1 1 0-96zM114.7 368a48 48 0 1 1 90.5 32 48 48 0 1 1 -90.5-32zM331.2 100.6c5.3 5.2 6.3 13.3 2.5 19.6L300.3 176l51.7 0c7.1 0 13.4 4.7 15.4 11.6s-.8 14.2-6.9 18l-128 80c-6.3 3.9-14.4 3-19.7-2.2s-6.3-13.3-2.5-19.6L243.7 208 192 208c-7.1 0-13.4-4.7-15.4-11.6s.8-14.2 6.9-18l128-80c6.3-3.9 14.4-3 19.7 2.2z";
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
    exports.faCarSideBolt = exports.definition;
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

// src/_entries/faCarSideBolt.js
var import_faCarSideBolt = __toESM(require_faCarSideBolt());
var export_faCarSideBolt = import_faCarSideBolt.definition;
export {
  export_faCarSideBolt as faCarSideBolt
};
