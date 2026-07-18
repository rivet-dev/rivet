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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSandwich.js
var require_faSandwich = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSandwich.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "sandwich";
    var width = 512;
    var height = 512;
    var aliases = [129386];
    var unicode = "f81f";
    var svgPathData = "M0 96C0 78.3 14.3 64 32 64l448 0c17.7 0 32 14.3 32 32l0 48c0 17.7-14.3 32-32 32L32 176c-17.7 0-32-14.3-32-32L0 96zM303.9 382.6c10 5.8 22.3 5.8 32.2 0l70.2-40.9c6.4-3.7 13.6-5.7 20.9-5.7l4.8 0 48 0c17.7 0 32 14.3 32 32l0 48c0 17.7-14.3 32-32 32L32 448c-17.7 0-32-14.3-32-32l0-48c0-17.7 14.3-32 32-32l176 0 4.8 0c7.4 0 14.6 2 20.9 5.7l70.2 40.9zM32 304c-17.7 0-32-14.3-32-32s14.3-32 32-32c7.2 0 15.8-3.7 35.4-13c16.6-7.8 40.6-19 68.6-19c30.6 0 53 10.2 70.1 18l1.9 .9c17.2 7.8 29.7 13.1 48.1 13.1s30.9-5.3 48.1-13.1l1.9-.9c17-7.8 39.4-18 70.1-18c28 0 52.1 11.3 68.6 19c19.6 9.2 28.2 13 35.4 13c17.7 0 32 14.3 32 32s-14.3 32-32 32c-22.7 0-44.3-10.3-58.8-17.2c0 0 0 0 0 0c-1.3-.6-2.6-1.2-3.8-1.8c-17.4-8.2-29.4-13-41.4-13c-16.2 0-27.5 5-45.4 13.1c0 0 0 0-.1 0c-18 8.2-41.4 18.9-74.5 18.9s-56.6-10.7-74.5-18.9c0 0 0 0-.1 0C163.5 277 152.2 272 136 272c-12 0-23.9 4.7-41.4 13c-1.2 .6-2.5 1.2-3.8 1.8C76.3 293.7 54.7 304 32 304z";
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
    exports.faSandwich = exports.definition;
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

// src/_entries/faSandwich.js
var import_faSandwich = __toESM(require_faSandwich());
var export_faSandwich = import_faSandwich.definition;
export {
  export_faSandwich as faSandwich
};
