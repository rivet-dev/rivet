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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBatteryBolt.js
var require_faBatteryBolt = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBatteryBolt.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "battery-bolt";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f376";
    var svgPathData = "M80 96C35.8 96 0 131.8 0 176L0 336c0 44.2 35.8 80 80 80l92.2 0c-12.9-14.5-16.1-35.6-7.2-53.5l5.3-10.5L80 352c-8.8 0-16-7.2-16-16l0-160c0-8.8 7.2-16 16-16l139.2 0 80-64L80 96zM464 352l-139.2 0-80 64L464 416c44.2 0 80-35.8 80-80l0-16c17.7 0 32-14.3 32-32l0-64c0-17.7-14.3-32-32-32l0-16c0-44.2-35.8-80-80-80l-92.2 0c13 14.5 16.1 35.6 7.2 53.5L373.7 160l90.3 0c8.8 0 16 7.2 16 16l0 160c0 8.8-7.2 16-16 16zM346.1 115.6c-5.8-4.7-14.2-4.7-20.1-.1l-160 128c-5.3 4.2-7.4 11.4-5.1 17.8s8.3 10.7 15.1 10.7l70.1 0L193.7 376.8c-3.4 6.7-1.6 14.9 4.3 19.6s14.2 4.7 20.1 .1l160-128c5.3-4.2 7.4-11.4 5.1-17.8s-8.3-10.7-15.1-10.7l-70.1 0 52.4-104.8c3.4-6.7 1.6-14.9-4.2-19.6z";
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
    exports.faBatteryBolt = exports.definition;
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

// src/_entries/faBatteryBolt.js
var import_faBatteryBolt = __toESM(require_faBatteryBolt());
var export_faBatteryBolt = import_faBatteryBolt.definition;
export {
  export_faBatteryBolt as faBatteryBolt
};
