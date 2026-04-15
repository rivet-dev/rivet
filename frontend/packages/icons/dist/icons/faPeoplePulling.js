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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPeoplePulling.js
var require_faPeoplePulling = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPeoplePulling.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "people-pulling";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e535";
    var svgPathData = "M80 96a48 48 0 1 0 0-96 48 48 0 1 0 0 96zM64 128c-35.3 0-64 28.7-64 64L0 320c0 17.7 14.3 32 32 32 9.8 0 18.5-4.4 24.4-11.2L80.4 485.3c2.9 17.4 19.4 29.2 36.8 26.3s29.2-19.4 26.3-36.8l-20.5-122.7 15.7 0 30 134.9c3.8 17.3 20.9 28.1 38.2 24.3s28.1-20.9 24.3-38.2l-57.3-258 116.3 53.8c.5 .3 1.1 .5 1.6 .7 8.6 3.6 18 3.1 25.9-.7 3.4-1.6 6.6-3.9 9.3-6.7 3.1-3.2 5.5-7 7.1-11.4 .1-.3 .2-.7 .3-1l2.5-7.5c5.7-17.1 18.3-30.9 34.7-38.2l8-3.5c1-.4 1.9-.8 2.9-1.2l-16.9 63.5c-5.6 21.1-.1 43.6 14.7 59.7l70.7 77.1 22 88.1c4.3 17.1 21.7 27.6 38.8 23.3s27.6-21.7 23.3-38.8l-23-92.1c-1.9-7.8-5.8-14.9-11.2-20.8l-49.5-54 19.3-65.5 9.6 23c4.4 10.6 12.5 19.3 22.8 24.5l26.7 13.3c15.8 7.9 35 1.5 42.9-14.3s1.5-35-14.3-42.9l-21.4-10.7-15.3-36.8c-17.1-41.1-57.3-67.9-101.9-67.9-22.8 0-45.3 4.8-66.1 14l-8 3.5c-24.4 10.9-44.6 29-58.1 51.6L157.3 136.9C144.7 131 130.9 128 117 128l-53 0zM464 96a48 48 0 1 0 0-96 48 48 0 1 0 0 96zM349.7 335.6l-25 62.4-59.4 59.4c-12.5 12.5-12.5 32.8 0 45.3s32.8 12.5 45.3 0L372.3 441c4.6-4.6 8.2-10.1 10.6-16.1l14.5-36.2-40.7-44.4c-2.5-2.7-4.8-5.6-7-8.6z";
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
    exports.faPeoplePulling = exports.definition;
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

// src/_entries/faPeoplePulling.js
var import_faPeoplePulling = __toESM(require_faPeoplePulling());
var export_faPeoplePulling = import_faPeoplePulling.definition;
export {
  export_faPeoplePulling as faPeoplePulling
};
