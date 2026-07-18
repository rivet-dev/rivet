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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBroomWide.js
var require_faBroomWide = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBroomWide.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "broom-wide";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e5d1";
    var svgPathData = "M502.6 9.4c-12.5-12.5-32.8-12.5-45.3 0l-128 128-15.8-15.8c-15.2-15.2-38.5-18.4-57.3-8l-32.5 18L380.4 288.1l18-32.5c10.4-18.7 7.1-42.1-8-57.3l-15.8-15.8 128-128c12.5-12.5 12.5-32.8 0-45.3zM187.5 151.8L16.4 246.9C6.3 252.5 0 263.2 0 274.8c0 8.5 3.4 16.6 9.3 22.6l43.2 43.2c2.1 2.1 5.3 2.9 8.2 1.9l52.1-17.4c6.3-2.1 12.2 3.9 10.1 10.1l-17.4 52.1c-1 2.9-.2 6 1.9 8.2L214.7 502.7c6 6 14.1 9.3 22.6 9.3c11.6 0 22.3-6.3 27.9-16.4l95.1-171.1L187.5 151.8z";
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
    exports.faBroomWide = exports.definition;
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

// src/_entries/faBroomWide.js
var import_faBroomWide = __toESM(require_faBroomWide());
var export_faBroomWide = import_faBroomWide.definition;
export {
  export_faBroomWide as faBroomWide
};
