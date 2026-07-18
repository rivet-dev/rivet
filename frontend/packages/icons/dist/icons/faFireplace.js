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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFireplace.js
var require_faFireplace = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFireplace.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "fireplace";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f79a";
    var svgPathData = "M0 32L0 64C0 81.7 14.3 96 32 96l576 0c17.7 0 32-14.3 32-32l0-32c0-17.7-14.3-32-32-32L32 0C14.3 0 0 14.3 0 32zm608 96L32 128l0 352c0 17.7 14.3 32 32 32l96 0 0-160c0-88.4 71.6-160 160-160s160 71.6 160 160l0 160 96 0c17.7 0 32-14.3 32-32l0-352zM338 299.4l-13.3-14c-1.4-1.5-2.8-3.1-4.2-4.7c0 0 0 0 0 0c-6.3-7.2-13.1-14.9-22.9-15.1c-5.1-.1-10.2 1.6-14.2 5.3c-16.4 14.8-35 34.2-49.6 56.1c-14.5 21.8-25.8 47-25.8 73.1c0 62 49.3 111.9 112 111.9c62.1 0 112-49.8 112-111.9c0-21-7.7-42.6-18.3-61.7C403 319.3 389 301.8 375.2 289c-3.9-3.6-8.9-5.5-13.9-5.5c-5.3 0-10.9 2-14.6 6.2l-8.6 9.7zm26.8 134.3c0 24.7-20.1 44.8-44.8 44.8s-44.8-20.1-44.8-44.8c0-25.6 25.9-51.1 38.3-61.9c3.8-3.3 9.2-3.3 12.9 0c12.4 10.8 38.3 36.3 38.3 61.9z";
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
    exports.faFireplace = exports.definition;
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

// src/_entries/faFireplace.js
var import_faFireplace = __toESM(require_faFireplace());
var export_faFireplace = import_faFireplace.definition;
export {
  export_faFireplace as faFireplace
};
