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

// src/node_modules/@fortawesome/free-solid-svg-icons/faMoneyBillWave.js
var require_faMoneyBillWave = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faMoneyBillWave.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "money-bill-wave";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f53a";
    var svgPathData = "M0 419.6L0 109.5c0-23.2 24.1-38.6 46.3-32 87.7 26.2 149.7 5.5 212.1-15.3 64.5-21.5 129.4-43.1 223.3-13.1 18.5 5.9 30.3 23.8 30.3 43.3l0 310.1c0 23.2-24.1 38.6-46.2 32-87.7-26.2-149.8-5.5-212.1 15.3-64.5 21.5-129.4 43.1-223.3 13.1-18.5-5.9-30.3-23.8-30.3-43.3zM336 256c0-53-35.8-96-80-96s-80 43-80 96 35.8 96 80 96 80-43 80-96zM120 413.6c4.4 0 7.9-3.8 7.2-8.1-4.6-27.8-27-49.5-55.2-53-4.4-.5-8 3.1-8 7.5l0 39.9c0 3.6 2.4 6.8 6 7.7 17.9 4.2 34.3 6.1 50 6.1zm318.5-51.1c5 .8 9.5-3 9.5-8l0-42.6c0-4.4-3.6-8.1-8-7.5-25.2 3.1-45.9 20.9-53.2 44.6-1.4 4.7 2.3 9.1 7.2 9.2 14.2 .4 29 1.7 44.4 4.3zM448 152l0-39.9c0-3.6-2.5-6.8-6-7.7-17.9-4.2-34.3-6.1-50-6.1-4.4 0-7.9 3.8-7.2 8.1 4.6 27.8 27 49.5 55.2 53 4.4 .5 8-3.1 8-7.5zM125.2 162.9c1.4-4.7-2.3-9.1-7.2-9.2-14.2-.4-29-1.7-44.4-4.3-5-.8-9.5 3-9.5 8L64 200c0 4.4 3.6 8.1 8 7.5 25.2-3.1 45.9-20.9 53.2-44.6z";
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
    exports.faMoneyBillWave = exports.definition;
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

// src/_entries/faMoneyBillWave.js
var import_faMoneyBillWave = __toESM(require_faMoneyBillWave());
var export_faMoneyBillWave = import_faMoneyBillWave.definition;
export {
  export_faMoneyBillWave as faMoneyBillWave
};
