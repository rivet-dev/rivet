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

// src/node_modules/@fortawesome/free-solid-svg-icons/faCrow.js
var require_faCrow = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faCrow.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "crow";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f520";
    var svgPathData = "M456.5 0c-48.6 0-88 39.4-88 88l0 36-355.2 266.4C-.8 401-3.7 421 6.9 435.2s30.6 17 44.8 6.4l76.8-57.6 131.3 0 46.6 113.1 1 2.2c5.7 10.7 18.8 15.5 30.3 10.8s17.3-17.3 13.9-29l-.8-2.3-39.1-94.9 40.9 0c1.1 0 2.2 0 3.2 0l46.6 113.2 1 2.2c5.7 10.7 18.8 15.5 30.3 10.8s17.3-17.3 13.9-29l-.8-2.3-42-102C485.3 354.1 544.5 280 544.5 192l0-72 80.5-20.1c8.6-2.1 13.8-10.8 11.6-19.4-7.1-28.5-32.7-48.5-62.1-48.5l-50.1 0C508.2 12.5 483.8 0 456.5 0zm0 64a24 24 0 1 1 0 48 24 24 0 1 1 0-48z";
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
    exports.faCrow = exports.definition;
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

// src/_entries/faCrow.js
var import_faCrow = __toESM(require_faCrow());
var export_faCrow = import_faCrow.definition;
export {
  export_faCrow as faCrow
};
