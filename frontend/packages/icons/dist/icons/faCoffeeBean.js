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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCoffeeBean.js
var require_faCoffeeBean = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCoffeeBean.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "coffee-bean";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e13e";
    var svgPathData = "M333.6 205.6c24.4-24.6 45.4-67.3 59.9-101.7c5.1-12.2 20.9-15.3 28.2-4.3c51.2 77.7 26.2 201.2-63.4 290.8c-79.6 79.6-186 108.1-263.2 77.7c-14.8-5.8-20-23.2-13.2-37.5c.7-1.6 1.5-3.1 2.2-4.7c16.5-34.5 37.5-72.2 57.4-92.3c29.7-29.9 59.7-44.9 91.8-61l.5-.3s0 0 0 0c32-15.9 66.1-32.9 99.7-66.8zM55.3 412c-5.3 11-19.9 13.6-26.9 3.6c-54-77.4-29.6-203.2 61.2-294C169.9 41.3 277.4 13 354.7 44.7c14 5.7 19.4 22 13.7 36c-1.4 3.5-2.9 7.1-4.4 10.8c-14.5 34.5-33.5 71.8-53.1 91.5c-29.7 29.9-59.7 44.9-91.8 61l-.5 .3c-32 15.9-66.1 32.9-99.7 66.8c-24.1 24.3-47.1 66.6-63.6 101z";
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
    exports.faCoffeeBean = exports.definition;
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

// src/_entries/faCoffeeBean.js
var import_faCoffeeBean = __toESM(require_faCoffeeBean());
var export_faCoffeeBean = import_faCoffeeBean.definition;
export {
  export_faCoffeeBean as faCoffeeBean
};
