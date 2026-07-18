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

// src/node_modules/@fortawesome/free-solid-svg-icons/faSterlingSign.js
var require_faSterlingSign = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faSterlingSign.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "sterling-sign";
    var width = 384;
    var height = 512;
    var aliases = [163, "gbp", "pound-sign"];
    var unicode = "f154";
    var svgPathData = "M91.3 288l-34.8 0c-13.3 0-24-10.7-24-24s10.7-24 24-24l21.4 0C37.3 147.3 105.1 42 207.6 42l8.2 0c33.6 0 66.2 11.3 92.5 32.2l16.1 12.7c13.9 11 16.2 31.1 5.2 45s-31.1 16.2-45 5.2l-16.1-12.7c-15-11.9-33.6-18.4-52.8-18.4l-8.2 0c-57.3 0-94.7 59.9-69.7 111.4 3.6 7.4 6.6 14.9 9.1 22.6l149.5 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-141.2 0c1 35.3-8.7 70.6-28.9 100.9l-18.1 27.1 212.2 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-272 0c-11.8 0-22.6-6.5-28.2-16.9s-5-23 1.6-32.9l51.2-76.8c13.1-19.6 19.2-42.6 18.2-65.4z";
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
    exports.faSterlingSign = exports.definition;
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

// src/_entries/faSterlingSign.js
var import_faSterlingSign = __toESM(require_faSterlingSign());
var export_faSterlingSign = import_faSterlingSign.definition;
export {
  export_faSterlingSign as faSterlingSign
};
