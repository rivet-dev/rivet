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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBowlChopsticks.js
var require_faBowlChopsticks = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBowlChopsticks.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bowl-chopsticks";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e2e9";
    var svgPathData = "M18.5 125.1l473.8-74c11.3-1.8 19.7-11.5 19.7-23c0-14.6-13.3-25.6-27.6-22.9L13.1 93.6C5.5 95 0 101.6 0 109.3c0 9.8 8.8 17.3 18.5 15.8zM0 159.7c0 9 7.3 16.2 16.2 16.2l472.1 0c13 0 23.6-10.6 23.6-23.6c0-13.3-11-24-24.4-23.6L15.7 143.5C7 143.8 0 151 0 159.7zM33.6 224C15.4 224 .9 239.3 2 257.5c5.4 91.7 59.7 169.7 136.4 209.7l1.2 9.7c2.5 20 19.5 35 39.7 35l153.4 0c20.2 0 37.2-15 39.7-35l1.2-9.7c76.8-40 131-118 136.4-209.7c1.1-18.2-13.4-33.5-31.6-33.5L33.6 224z";
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
    exports.faBowlChopsticks = exports.definition;
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

// src/_entries/faBowlChopsticks.js
var import_faBowlChopsticks = __toESM(require_faBowlChopsticks());
var export_faBowlChopsticks = import_faBowlChopsticks.definition;
export {
  export_faBowlChopsticks as faBowlChopsticks
};
