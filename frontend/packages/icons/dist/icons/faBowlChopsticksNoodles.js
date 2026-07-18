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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBowlChopsticksNoodles.js
var require_faBowlChopsticksNoodles = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBowlChopsticksNoodles.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bowl-chopsticks-noodles";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e2ea";
    var svgPathData = "M13.1 93.6L128 72l0-16c0-13.3 10.7-24 24-24s24 10.7 24 24l0 7 0 37.5 0 38 0 37.5 0 48s0 0 0 0l32 0s0 0 0 0l0-48 0-38.5 0-42L208 57l0-17c0-13.3 10.7-24 24-24s24 10.7 24 24l0 8 0 40 0 48 0 40 0 48s0 0 0 0l32 0s0 0 0 0l190.4 0c18.2 0 32.7 15.3 31.6 33.5c-5.4 91.7-59.7 169.7-136.4 209.7l-1.2 9.7c-2.5 20-19.5 35-39.7 35l-153.4 0c-20.2 0-37.2-15-39.7-35l-1.2-9.7C61.6 427.2 7.4 349.3 2 257.5C.9 239.3 15.4 224 33.6 224l94.4 0 0-48L16.2 176C7.3 176 0 168.7 0 159.7c0-8.8 7-16 15.7-16.2L128 140l0-32L18.5 125.1C8.8 126.6 0 119.1 0 109.3C0 101.6 5.5 95 13.1 93.6zM488.4 176L288 176l0-41 199.6-6.2C501 128.3 512 139 512 152.4c0 13-10.6 23.6-23.6 23.6zm3.9-124.9L288 83l0-41L484.4 5.2C498.7 2.5 512 13.5 512 28.1c0 11.5-8.4 21.2-19.7 23z";
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
    exports.faBowlChopsticksNoodles = exports.definition;
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

// src/_entries/faBowlChopsticksNoodles.js
var import_faBowlChopsticksNoodles = __toESM(require_faBowlChopsticksNoodles());
var export_faBowlChopsticksNoodles = import_faBowlChopsticksNoodles.definition;
export {
  export_faBowlChopsticksNoodles as faBowlChopsticksNoodles
};
