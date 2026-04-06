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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faChopsticks.js
var require_faChopsticks = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faChopsticks.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "chopsticks";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e3f7";
    var svgPathData = "M368 0c-15.3 0-29 7.2-37.7 18.3L3.4 486.1c-5.5 6.9-4.3 17 2.7 22.5s17 4.2 22.5-2.7L405.7 77.7C412.2 69.5 416 59.2 416 48c0-26.5-21.5-48-48-48zM131.9 506.4c5.8 6.7 15.9 7.5 22.6 1.7L623.2 148.4c10.3-8.8 16.8-21.9 16.8-36.4c0-26.5-21.5-48-48-48c-11.9 0-22.8 4.4-31.2 11.6L133.6 483.9c-6.7 5.8-7.5 15.8-1.7 22.6z";
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
    exports.faChopsticks = exports.definition;
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

// src/_entries/faChopsticks.js
var import_faChopsticks = __toESM(require_faChopsticks());
var export_faChopsticks = import_faChopsticks.definition;
export {
  export_faChopsticks as faChopsticks
};
