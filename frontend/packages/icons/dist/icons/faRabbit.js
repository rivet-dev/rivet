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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faRabbit.js
var require_faRabbit = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faRabbit.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "rabbit";
    var width = 512;
    var height = 512;
    var aliases = [128007];
    var unicode = "f708";
    var svgPathData = "M321.4 128c4.8 0 9.6 .3 14.3 .9c1.1-24-2.6-47.9-10.9-70.1c7.6-17.8 18.6-34.2 32.7-48.3l4.6-4.6c3.7-3.7 8.8-5.8 14-5.8c9.2 0 17.2 6.4 19.3 15.4l1 4.4c9.8 42.4 1.8 86.5-21 122.4c5.5 3.1 10.7 6.6 15.6 10.6L425.8 181c14 11.4 22.2 28.6 22.2 46.7c0 33.3-27 60.3-60.3 60.3L352 288l0 25.8 61.7 154.3c6.6 16.4-1.4 35-17.8 41.6s-35-1.4-41.6-17.8l-39.4-98.6L265.6 448l22.4 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-80 0-48 0c-28.5 0-52.6-18.6-60.9-44.2C88.8 480.1 73.3 488 56 488c-30.9 0-56-25.1-56-56s25.1-56 56-56c17.4 0 33 8 43.3 20.5C114 317.7 176.8 255.9 256 242.7l0-49.2c0-10.9 2.7-21.2 7.4-30.2c-34.3-36.5-50.3-86.8-43.2-136.8l.7-4.9C222.7 9.2 233.2 0 245.7 0c6.6 0 12.9 2.6 17.6 7.3l3.1 3.1c31.8 31.5 47.9 74.8 45.4 118.4c3.1-.5 6.3-.7 9.6-.7zM368 224a16 16 0 1 0 0-32 16 16 0 1 0 0 32z";
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
    exports.faRabbit = exports.definition;
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

// src/_entries/faRabbit.js
var import_faRabbit = __toESM(require_faRabbit());
var export_faRabbit = import_faRabbit.definition;
export {
  export_faRabbit as faRabbit
};
