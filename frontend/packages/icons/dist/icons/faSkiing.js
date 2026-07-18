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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPersonSkiing.js
var require_faPersonSkiing = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPersonSkiing.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "person-skiing";
    var width = 576;
    var height = 512;
    var aliases = [9975, "skiing"];
    var unicode = "f7c9";
    var svgPathData = "M416 56a56 56 0 1 1 112 0 56 56 0 1 1 -112 0zM2.7 300.9c6.1-11.8 20.6-16.3 32.4-10.2l197.8 102.7 62.3-87.2-82-92.2c-11-12.4-16.2-27.5-16.2-42.4l111.6 53.4 42.9 48.2c14.9 16.7 16.2 41.6 3.2 59.8l-64.4 90.2 128.3 66.6c13.6 7.1 29.8 7.2 43.6 .3l15.2-7.6c11.9-5.9 26.3-1.1 32.2 10.7s1.1 26.3-10.7 32.2l-15.2 7.6c-27.5 13.7-59.9 13.5-87.2-.7L12.9 333.3C1.2 327.2-3.4 312.7 2.7 300.9zM103 49.6l18 8.7 8.7-17.4c4-7.9 13.6-11.1 21.5-7.2s11.1 13.6 7.2 21.5l-8.5 17 84.8 41 .4-.2 76.1-33.8c31.3-13.9 67.9-.7 83.2 29.9l28.9 57.8 68.7 27.5c16.4 6.6 24.4 25.2 17.8 41.6s-25.2 24.4-41.6 17.8L393.8 224c-10.9-4.4-19.8-12.6-25.1-23.1l-11.5-23.1c-16.4 9.4-25.9 14.8-28.5 16.3l-7.6-3.7-185.6-89.6-9.2 18.3c-4 7.9-13.6 11.1-21.5 7.2s-11.1-13.6-7.2-21.5l9-17.9-17.6-8.5C81.1 74.6 77.8 65 81.6 57S95 45.7 103 49.6z";
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
    exports.faPersonSkiing = exports.definition;
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

// src/_entries/faSkiing.js
var import_faPersonSkiing = __toESM(require_faPersonSkiing());
var export_faSkiing = import_faPersonSkiing.definition;
export {
  export_faSkiing as faSkiing
};
