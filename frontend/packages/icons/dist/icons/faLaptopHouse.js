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

// src/node_modules/@fortawesome/free-solid-svg-icons/faHouseLaptop.js
var require_faHouseLaptop = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faHouseLaptop.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "house-laptop";
    var width = 640;
    var height = 512;
    var aliases = ["laptop-house"];
    var unicode = "e066";
    var svgPathData = "M448 240l19.9 0c15.5 0 28.1-12.6 28.1-28.1 0-7.6-3.1-14.9-8.6-20.2L283.5-4.9C276.1-12 266.3-16 256-16s-20.1 4-27.5 11.1L24.6 191.7C19.1 197 16 204.3 16 211.9 16 227.4 28.6 240 44.1 240l19.9 0 0 144c0 35.3 28.7 64 64 64l85.7 0c7.4-6.6 16.4-11.4 26.3-14l0-130c0-5.5 .7-10.9 2-16l-10 0c-13.3 0-24-10.7-24-24l0-48c0-13.3 10.7-24 24-24l48 0c13.3 0 24 10.7 24 24l0 24 144 0zM352 352l160 0 0 128-160 0 0-128zm-64-32l0 160-32 0c-8.8 0-16 7.2-16 16 0 26.5 21.5 48 48 48l288 0c26.5 0 48-21.5 48-48 0-8.8-7.2-16-16-16l-32 0 0-160c0-17.7-14.3-32-32-32l-224 0c-17.7 0-32 14.3-32 32z";
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
    exports.faHouseLaptop = exports.definition;
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

// src/_entries/faLaptopHouse.js
var import_faHouseLaptop = __toESM(require_faHouseLaptop());
var export_faLaptopHouse = import_faHouseLaptop.definition;
export {
  export_faLaptopHouse as faLaptopHouse
};
