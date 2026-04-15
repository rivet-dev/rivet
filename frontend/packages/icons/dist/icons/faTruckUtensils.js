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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTruckUtensils.js
var require_faTruckUtensils = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTruckUtensils.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "truck-utensils";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e628";
    var svgPathData = "M48 0C21.5 0 0 21.5 0 48L0 368c0 26.5 21.5 48 48 48l16 0c0 53 43 96 96 96s96-43 96-96l128 0c0 53 43 96 96 96s96-43 96-96l32 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l0-64 0-32 0-18.7c0-17-6.7-33.3-18.7-45.3L512 114.7c-12-12-28.3-18.7-45.3-18.7L416 96l0-48c0-26.5-21.5-48-48-48L48 0zM416 160l50.7 0L544 237.3l0 18.7-128 0 0-96zM112 416a48 48 0 1 1 96 0 48 48 0 1 1 -96 0zm368-48a48 48 0 1 1 0 96 48 48 0 1 1 0-96zM89.5 77.5c.7-5.9 5.6-10.3 11.6-10.3c6.2 0 11.3 4.8 11.6 11l4.4 75.5c.2 3 2.6 5.3 5.6 5.3s5.5-2.3 5.6-5.3l4.4-75.8c.3-6 5.3-10.6 11.3-10.6s10.9 4.7 11.3 10.6l4.4 75.8c.2 3 2.6 5.3 5.6 5.3s5.5-2.3 5.6-5.3l4.4-75.5c.4-6.2 5.5-11 11.6-11c5.9 0 10.9 4.4 11.6 10.3l9.5 81.4c0 29.4-20.5 54-48 60.3l0 84.8c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-84.8c-27.5-6.3-48-30.9-48-60.3l9.5-81.4zM256 154.9c0-63.1 41.3-86 50.3-90.2c1.1-.5 2.3-.8 3.6-.8c5.6 0 10.1 4.5 10.1 10.1l0 53.9 0 96 0 80c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-80c-17.7 0-32-14.3-32-32l0-37z";
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
    exports.faTruckUtensils = exports.definition;
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

// src/_entries/faTruckUtensils.js
var import_faTruckUtensils = __toESM(require_faTruckUtensils());
var export_faTruckUtensils = import_faTruckUtensils.definition;
export {
  export_faTruckUtensils as faTruckUtensils
};
