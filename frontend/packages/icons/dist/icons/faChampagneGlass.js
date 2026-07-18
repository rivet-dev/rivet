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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faChampagneGlass.js
var require_faChampagneGlass = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faChampagneGlass.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "champagne-glass";
    var width = 320;
    var height = 512;
    var aliases = ["glass-champagne"];
    var unicode = "f79e";
    var svgPathData = "M88 0C71.9 0 58.3 12 56.3 27.9L33.2 208C24.9 273.4 67.3 332.4 128 348l0 100-48 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l80 0 80 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-48 0 0-100c60.7-15.6 103.1-74.6 94.8-140l-23-180.1C261.7 12 248.1 0 232 0L88 0zm20 128l8.2-64 87.7 0 8.2 64-104 0z";
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
    exports.faChampagneGlass = exports.definition;
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

// src/_entries/faChampagneGlass.js
var import_faChampagneGlass = __toESM(require_faChampagneGlass());
var export_faChampagneGlass = import_faChampagneGlass.definition;
export {
  export_faChampagneGlass as faChampagneGlass
};
