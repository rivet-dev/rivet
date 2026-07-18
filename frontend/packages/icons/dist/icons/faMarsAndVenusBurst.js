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

// src/node_modules/@fortawesome/free-solid-svg-icons/faMarsAndVenusBurst.js
var require_faMarsAndVenusBurst = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faMarsAndVenusBurst.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "mars-and-venus-burst";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e523";
    var svgPathData = "M464-32c0 17.7 14.3 32 32 32l34.7 0-63 63c-28.4-19.5-62.7-31-99.8-31-97.2 0-176 78.8-176 176 0 86.3 62.1 158.1 144 173.1l0 34.9-32 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l32 0 0 32c0 17.7 14.3 32 32 32s32-14.3 32-32l0-32 32 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-32 0 0-34.9c81.9-15 144-86.8 144-173.1 0-37-11.4-71.4-31-99.8l63-63 0 34.7c0 17.7 14.3 32 32 32s32-14.3 32-32l0-112c0-17.7-14.3-32-32-32L496-64c-17.7 0-32 14.3-32 32zM368 96a112 112 0 1 1 0 224 112 112 0 1 1 0-224zM190.9-21.9C188.4-28 182.6-32 176-32s-12.4 4-14.9 10.1l-29.4 74-76.2-23.1c-6.3-1.9-13.1 .2-17.2 5.3S33.8 46.5 37 52.2l39.5 69.1-65.6 45.1c-5.4 3.7-8 10.3-6.5 16.7s6.7 11.2 13.1 12.2l78.7 12.2-5.6 79.4c-.5 6.5 3.1 12.7 9 15.5s12.9 1.8 17.8-2.6L152.1 268c-5.3-19.1-8.1-39.2-8.1-60 0-65.4 28-124.2 72.6-165.1L190.9-21.9z";
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
    exports.faMarsAndVenusBurst = exports.definition;
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

// src/_entries/faMarsAndVenusBurst.js
var import_faMarsAndVenusBurst = __toESM(require_faMarsAndVenusBurst());
var export_faMarsAndVenusBurst = import_faMarsAndVenusBurst.definition;
export {
  export_faMarsAndVenusBurst as faMarsAndVenusBurst
};
